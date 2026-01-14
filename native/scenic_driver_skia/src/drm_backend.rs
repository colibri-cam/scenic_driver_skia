use std::collections::HashMap;
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::os::raw::c_void;
use std::ptr;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::Duration;

use drm::ClientCapability;
use drm::Device as BasicDevice;
use drm::control::{
    self, AtomicCommitFlags, Device as ControlDevice, Event, PlaneType, ResourceHandles, atomic,
    connector, crtc, framebuffer, plane, property,
};
use gbm::{
    AsRaw, BufferObject, BufferObjectFlags, Device as GbmDevice, Format as GbmFormat, Surface,
};
use glutin_egl_sys::egl;
use glutin_egl_sys::egl::types::{EGLConfig, EGLContext, EGLDisplay, EGLSurface, EGLenum, EGLint};
use libloading::Library;
use skia_safe::{Color, Paint, PaintStyle, gpu::gl::FramebufferInfo};

use crate::cursor::CursorState;
use crate::drm_input::DrmInput;
use crate::input::{InputEvent, InputQueue, notify_input_ready};
use crate::renderer::{RenderState, Renderer};

const EGL_PLATFORM_GBM_KHR: EGLenum = 0x31D7;

struct Card(File);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for Card {
    fn as_raw_fd(&self) -> i32 {
        self.0.as_raw_fd()
    }
}

impl BasicDevice for Card {}
impl ControlDevice for Card {}

struct EglState {
    egl: egl::Egl,
    _egl_lib: Library,
    display: EGLDisplay,
    _context: EGLContext,
    surface: EGLSurface,
}

fn open_card() -> Result<Card, String> {
    let card_path =
        std::env::var("SCENIC_DRM_CARD").unwrap_or_else(|_| String::from("/dev/dri/card0"));

    let fd = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&card_path)
        .map_err(|e| format!("failed to open {card_path}: {e}"))?;

    Ok(Card(fd))
}

fn mode_distance(mode: &control::Mode, requested: (u32, u32)) -> i64 {
    let (width, height) = mode.size();
    let dx = width as i64 - requested.0 as i64;
    let dy = height as i64 - requested.1 as i64;
    dx * dx + dy * dy
}

fn choose_mode(
    modes: &[control::Mode],
    requested: Option<(u32, u32)>,
) -> Result<control::Mode, String> {
    let first = modes
        .first()
        .cloned()
        .ok_or_else(|| "connector has no modes".to_string())?;
    let Some(requested) = requested else {
        return Ok(first);
    };

    let mut best = first;
    let mut best_score = mode_distance(&best, requested);
    for mode in modes.iter().skip(1) {
        let score = mode_distance(mode, requested);
        if score < best_score {
            best = *mode;
            best_score = score;
        }
    }
    Ok(best)
}

fn first_connected_connector(
    card: &Card,
    resources: &ResourceHandles,
    requested: Option<(u32, u32)>,
) -> Result<(connector::Handle, control::Mode, crtc::Handle), String> {
    for handle in resources.connectors() {
        let info = card
            .get_connector(*handle, false)
            .map_err(|e| format!("failed to read connector {handle:?}: {e}"))?;

        if info.state() != connector::State::Connected {
            continue;
        }

        let mode = choose_mode(info.modes(), requested)
            .map_err(|err| format!("connector {handle:?} {err}"))?;

        let crtc = resources
            .crtcs()
            .first()
            .copied()
            .ok_or_else(|| "no available CRTCs".to_string())?;

        return Ok((*handle, mode, crtc));
    }

    Err("no connected DRM connectors found".into())
}

fn is_primary_plane(card: &Card, plane: plane::Handle) -> Result<bool, String> {
    let props = card
        .get_properties(plane)
        .map_err(|e| format!("failed to get plane properties: {e}"))?;
    for (&id, &val) in props.iter() {
        let info = card
            .get_property(id)
            .map_err(|e| format!("failed to read property info: {e}"))?;
        if info
            .name()
            .to_str()
            .map(|name| name == "type")
            .unwrap_or(false)
        {
            return Ok(val == (PlaneType::Primary as u32).into());
        }
    }
    Ok(false)
}

fn find_primary_plane(
    card: &Card,
    resources: &ResourceHandles,
    crtc_handle: crtc::Handle,
) -> Result<plane::Handle, String> {
    let planes = card
        .plane_handles()
        .map_err(|e| format!("could not list planes: {e}"))?;
    let mut compatible = Vec::new();
    let mut primary = Vec::new();

    for plane in planes {
        let info = card
            .get_plane(plane)
            .map_err(|e| format!("failed to read plane info: {e}"))?;
        let compatible_crtcs = resources.filter_crtcs(info.possible_crtcs());
        if !compatible_crtcs.contains(&crtc_handle) {
            continue;
        }
        compatible.push(plane);
        if is_primary_plane(card, plane)? {
            primary.push(plane);
        }
    }

    primary
        .first()
        .copied()
        .or_else(|| compatible.first().copied())
        .ok_or_else(|| "no compatible planes found".to_string())
}

fn prop_handle(
    props: &HashMap<String, property::Info>,
    name: &str,
) -> Result<property::Handle, String> {
    props
        .get(name)
        .map(|info| info.handle())
        .ok_or_else(|| format!("missing property {name}"))
}

fn add_plane_properties(
    req: &mut atomic::AtomicModeReq,
    plane: plane::Handle,
    plane_props: &HashMap<String, property::Info>,
    crtc_handle: crtc::Handle,
    fb: framebuffer::Handle,
) -> Result<(), String> {
    req.add_property(
        plane,
        prop_handle(plane_props, "FB_ID")?,
        property::Value::Framebuffer(Some(fb)),
    );
    req.add_property(
        plane,
        prop_handle(plane_props, "CRTC_ID")?,
        property::Value::CRTC(Some(crtc_handle)),
    );
    Ok(())
}

fn add_plane_geometry(
    req: &mut atomic::AtomicModeReq,
    plane: plane::Handle,
    plane_props: &HashMap<String, property::Info>,
    mode: &control::Mode,
) -> Result<(), String> {
    let (width, height) = mode.size();
    req.add_property(
        plane,
        prop_handle(plane_props, "SRC_X")?,
        property::Value::UnsignedRange(0),
    );
    req.add_property(
        plane,
        prop_handle(plane_props, "SRC_Y")?,
        property::Value::UnsignedRange(0),
    );
    req.add_property(
        plane,
        prop_handle(plane_props, "SRC_W")?,
        property::Value::UnsignedRange((width as u64) << 16),
    );
    req.add_property(
        plane,
        prop_handle(plane_props, "SRC_H")?,
        property::Value::UnsignedRange((height as u64) << 16),
    );
    req.add_property(
        plane,
        prop_handle(plane_props, "CRTC_X")?,
        property::Value::SignedRange(0),
    );
    req.add_property(
        plane,
        prop_handle(plane_props, "CRTC_Y")?,
        property::Value::SignedRange(0),
    );
    req.add_property(
        plane,
        prop_handle(plane_props, "CRTC_W")?,
        property::Value::UnsignedRange(width as u64),
    );
    req.add_property(
        plane,
        prop_handle(plane_props, "CRTC_H")?,
        property::Value::UnsignedRange(height as u64),
    );
    Ok(())
}

fn wait_for_page_flip(card: &Card) -> Result<(), String> {
    loop {
        let events = card
            .receive_events()
            .map_err(|e| format!("failed to read DRM events: {e}"))?;
        for event in events {
            if matches!(event, Event::PageFlip(_)) {
                return Ok(());
            }
        }
    }
}

fn load_egl() -> Result<(Library, egl::Egl), String> {
    let lib = unsafe { Library::new("libEGL.so.1") }
        .map_err(|e| format!("failed to load libEGL: {e}"))?;
    let get_proc = unsafe {
        lib.get::<unsafe extern "system" fn(*const i8) -> *const c_void>(b"eglGetProcAddress\0")
            .map_err(|e| format!("failed to load eglGetProcAddress: {e}"))?
    };

    let egl = egl::Egl::load_with(|name| unsafe {
        let symbol = CString::new(name).expect("egl symbol");
        let ptr = get_proc(symbol.as_ptr());
        if !ptr.is_null() {
            return ptr;
        }
        let raw = format!("{name}\0");
        lib.get::<*const c_void>(raw.as_bytes())
            .map(|s| *s)
            .unwrap_or(ptr::null())
    });

    Ok((lib, egl))
}

fn egl_get_platform_display(egl: &egl::Egl, display_ptr: *mut c_void) -> EGLDisplay {
    if egl.GetPlatformDisplayEXT.is_loaded() {
        unsafe { egl.GetPlatformDisplayEXT(EGL_PLATFORM_GBM_KHR, display_ptr, ptr::null()) }
    } else if egl.GetPlatformDisplay.is_loaded() {
        unsafe { egl.GetPlatformDisplay(EGL_PLATFORM_GBM_KHR, display_ptr, ptr::null()) }
    } else {
        unsafe { egl.GetDisplay(display_ptr as egl::EGLNativeDisplayType) }
    }
}

fn init_egl(
    egl: &egl::Egl,
    gbm_device_ptr: *mut c_void,
    gbm_surface_ptr: *mut c_void,
) -> Result<(EGLDisplay, EGLContext, EGLSurface), String> {
    let display = egl_get_platform_display(egl, gbm_device_ptr);
    if display == egl::NO_DISPLAY {
        return Err("failed to get EGL display".to_string());
    }

    let mut major: EGLint = 0;
    let mut minor: EGLint = 0;
    if unsafe { egl.Initialize(display, &mut major, &mut minor) } == egl::FALSE {
        return Err("failed to initialize EGL".to_string());
    }

    if unsafe { egl.BindAPI(egl::OPENGL_ES_API) } == egl::FALSE {
        return Err("failed to bind EGL OpenGL ES API".to_string());
    }

    let config_attribs: [EGLint; 13] = [
        egl::SURFACE_TYPE as EGLint,
        egl::WINDOW_BIT as EGLint,
        egl::RENDERABLE_TYPE as EGLint,
        egl::OPENGL_ES2_BIT as EGLint,
        egl::RED_SIZE as EGLint,
        8,
        egl::GREEN_SIZE as EGLint,
        8,
        egl::BLUE_SIZE as EGLint,
        8,
        egl::ALPHA_SIZE as EGLint,
        8,
        egl::NONE as EGLint,
    ];

    let mut config: EGLConfig = ptr::null();
    let mut num_configs: EGLint = 0;
    if unsafe {
        egl.ChooseConfig(
            display,
            config_attribs.as_ptr(),
            &mut config,
            1,
            &mut num_configs,
        )
    } == egl::FALSE
        || num_configs == 0
    {
        return Err("failed to choose EGL config".to_string());
    }

    let context_attribs: [EGLint; 3] = [
        egl::CONTEXT_CLIENT_VERSION as EGLint,
        2,
        egl::NONE as EGLint,
    ];
    let context =
        unsafe { egl.CreateContext(display, config, egl::NO_CONTEXT, context_attribs.as_ptr()) };
    if context == egl::NO_CONTEXT {
        return Err("failed to create EGL context".to_string());
    }

    let surface = unsafe {
        egl.CreateWindowSurface(
            display,
            config,
            gbm_surface_ptr as egl::EGLNativeWindowType,
            ptr::null(),
        )
    };
    if surface == egl::NO_SURFACE {
        return Err("failed to create EGL surface".to_string());
    }

    if unsafe { egl.MakeCurrent(display, surface, surface, context) } == egl::FALSE {
        return Err("failed to make EGL context current".to_string());
    }

    unsafe {
        egl.SwapInterval(display, 1);
    }

    Ok((display, context, surface))
}

fn create_renderer(
    egl: &egl::Egl,
    dimensions: (u32, u32),
    text: String,
) -> Result<Renderer, String> {
    gl::load_with(|s| unsafe {
        let symbol = CString::new(s).expect("gl symbol");
        egl.GetProcAddress(symbol.as_ptr()) as *const _
    });

    let interface = skia_safe::gpu::gl::Interface::new_load_with(|name| unsafe {
        if name == "eglGetCurrentDisplay" {
            return ptr::null();
        }
        let symbol = CString::new(name).expect("egl symbol");
        egl.GetProcAddress(symbol.as_ptr()) as *const _
    })
    .ok_or_else(|| "could not create Skia GL interface".to_string())?;

    let gr_context = skia_safe::gpu::direct_contexts::make_gl(interface, None)
        .ok_or_else(|| "make_gl failed: could not create Skia direct context".to_string())?;

    let fb_info = {
        let mut fboid: i32 = 0;
        unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };
        FramebufferInfo {
            fboid: fboid as u32,
            format: skia_safe::gpu::gl::Format::RGBA8.into(),
            ..Default::default()
        }
    };

    Ok(Renderer::new(dimensions, fb_info, gr_context, 0, 0, text))
}

fn framebuffer_for_bo(
    card: &Card,
    cache: &mut HashMap<u32, framebuffer::Handle>,
    bo: &BufferObject<()>,
) -> Result<framebuffer::Handle, String> {
    let handle = unsafe { bo.handle().u32_ };
    if let Some(existing) = cache.get(&handle).copied() {
        return Ok(existing);
    }

    let framebuffer = card
        .add_framebuffer(bo, 24, 32)
        .map_err(|e| format!("failed to create framebuffer: {e}"))?;
    cache.insert(handle, framebuffer);
    Ok(framebuffer)
}

fn cursor_snapshot(cursor_state: &Arc<Mutex<CursorState>>) -> CursorState {
    cursor_state
        .lock()
        .map(|state| *state)
        .unwrap_or_else(|_| CursorState::new())
}

fn draw_software_cursor(renderer: &mut Renderer, cursor_pos: (f32, f32), screen_size: (u32, u32)) {
    let (width, height) = screen_size;
    let x = cursor_pos.0.clamp(0.0, width.saturating_sub(1) as f32);
    let y = cursor_pos.1.clamp(0.0, height.saturating_sub(1) as f32);

    let canvas = renderer.surface_mut().canvas();
    let mut fill = Paint::default();
    fill.set_anti_alias(true);
    fill.set_color(Color::from_argb(240, 255, 255, 255));
    canvas.draw_circle((x, y), 4.0, &fill);

    let mut stroke = Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_style(PaintStyle::Stroke);
    stroke.set_stroke_width(1.0);
    stroke.set_color(Color::from_argb(200, 0, 0, 0));
    canvas.draw_circle((x, y), 4.0, &stroke);
}

#[derive(Clone)]
pub struct DrmRunConfig {
    pub requested_size: Option<(u32, u32)>,
    pub cursor_state: Arc<Mutex<CursorState>>,
}

pub fn run(
    stop: Arc<AtomicBool>,
    text: Arc<Mutex<String>>,
    dirty: Arc<AtomicBool>,
    render_state: Arc<Mutex<RenderState>>,
    input_mask: Arc<AtomicU32>,
    input_events: Arc<Mutex<InputQueue>>,
    config: DrmRunConfig,
) {
    let card = match open_card() {
        Ok(card) => card,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    if let Err(e) = card.set_client_capability(ClientCapability::UniversalPlanes, true) {
        eprintln!("DRM backend unavailable: {e}");
        return;
    }
    if let Err(e) = card.set_client_capability(ClientCapability::Atomic, true) {
        eprintln!("DRM backend unavailable: {e}");
        return;
    }

    let resources = match card.resource_handles() {
        Ok(handles) => handles,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let (connector, mode, crtc_handle) =
        match first_connected_connector(&card, &resources, config.requested_size) {
            Ok(values) => values,
            Err(e) => {
                eprintln!("DRM backend unavailable: {e}");
                return;
            }
        };

    let plane = match find_primary_plane(&card, &resources, crtc_handle) {
        Ok(handle) => handle,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let con_props = match card
        .get_properties(connector)
        .and_then(|props| props.as_hashmap(&card))
    {
        Ok(props) => props,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };
    let crtc_props = match card
        .get_properties(crtc_handle)
        .and_then(|props| props.as_hashmap(&card))
    {
        Ok(props) => props,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };
    let plane_props = match card
        .get_properties(plane)
        .and_then(|props| props.as_hashmap(&card))
    {
        Ok(props) => props,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let (width, height) = mode.size();
    let dimensions = (width as u32, height as u32);
    if let Some(requested) = config.requested_size
        && requested != dimensions
        && let Ok(mut queue) = input_events.lock()
    {
        let notify = queue.push_event(InputEvent::ViewportReshape {
            width: dimensions.0,
            height: dimensions.1,
        });
        if let Some(pid) = notify {
            notify_input_ready(pid);
        }
    }
    let mut input = DrmInput::new(
        dimensions,
        Arc::clone(&input_mask),
        input_events,
        Arc::clone(&config.cursor_state),
    );

    let gbm_device = match GbmDevice::new(card.as_fd()) {
        Ok(device) => device,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let gbm_surface: Surface<()> = match gbm_device.create_surface(
        dimensions.0,
        dimensions.1,
        GbmFormat::Xrgb8888,
        BufferObjectFlags::SCANOUT | BufferObjectFlags::RENDERING,
    ) {
        Ok(surface) => surface,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let (egl_lib, egl_api) = match load_egl() {
        Ok(values) => values,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let (display, context, surface) = match init_egl(
        &egl_api,
        gbm_device.as_raw() as *mut c_void,
        gbm_surface.as_raw() as *mut c_void,
    ) {
        Ok(values) => values,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let egl_state = EglState {
        egl: egl_api,
        _egl_lib: egl_lib,
        display,
        _context: context,
        surface,
    };

    let mut renderer = match create_renderer(&egl_state.egl, dimensions, String::new()) {
        Ok(renderer) => renderer,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let mode_blob = match card.create_property_blob(&mode) {
        Ok(blob) => blob,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let mut framebuffer_cache: HashMap<u32, framebuffer::Handle> = HashMap::new();

    let initial_text = text.lock().unwrap_or_else(|e| e.into_inner()).clone();
    renderer.set_text(initial_text);
    if let Ok(state) = render_state.lock() {
        renderer.redraw(&state);
    }
    let mut cursor = cursor_snapshot(&config.cursor_state);
    if cursor.visible {
        draw_software_cursor(&mut renderer, cursor.pos, dimensions);
    }

    if unsafe {
        egl_state
            .egl
            .SwapBuffers(egl_state.display, egl_state.surface)
    } == egl::FALSE
    {
        eprintln!("DRM backend unavailable: eglSwapBuffers failed");
        return;
    }

    let bo = match unsafe { gbm_surface.lock_front_buffer() } {
        Ok(bo) => bo,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let fb = match framebuffer_for_bo(&card, &mut framebuffer_cache, &bo) {
        Ok(fb) => fb,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let mut atomic_req = atomic::AtomicModeReq::new();
    if let Err(e) = (|| -> Result<(), String> {
        atomic_req.add_property(
            connector,
            prop_handle(&con_props, "CRTC_ID")?,
            property::Value::CRTC(Some(crtc_handle)),
        );
        atomic_req.add_property(crtc_handle, prop_handle(&crtc_props, "MODE_ID")?, mode_blob);
        atomic_req.add_property(
            crtc_handle,
            prop_handle(&crtc_props, "ACTIVE")?,
            property::Value::Boolean(true),
        );
        add_plane_properties(&mut atomic_req, plane, &plane_props, crtc_handle, fb)?;
        add_plane_geometry(&mut atomic_req, plane, &plane_props, &mode)
    })() {
        eprintln!("DRM backend unavailable: {e}");
        return;
    }

    if let Err(e) = card.atomic_commit(AtomicCommitFlags::ALLOW_MODESET, atomic_req) {
        eprintln!("DRM backend unavailable: {e}");
        return;
    }

    let mut current_bo = Some(bo);
    let mut last_cursor = cursor;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        input.poll();
        cursor = cursor_snapshot(&config.cursor_state);
        if cursor.visible && cursor.pos != last_cursor.pos {
            dirty.store(true, Ordering::Relaxed);
        }
        if cursor.visible != last_cursor.visible {
            dirty.store(true, Ordering::Relaxed);
        }
        last_cursor = cursor;
        if dirty.swap(false, Ordering::Relaxed) {
            let updated = text.lock().unwrap_or_else(|e| e.into_inner()).clone();
            renderer.set_text(updated);
            if let Ok(state) = render_state.lock() {
                renderer.redraw(&state);
            }
            if cursor.visible {
                draw_software_cursor(&mut renderer, cursor.pos, dimensions);
            }

            if unsafe {
                egl_state
                    .egl
                    .SwapBuffers(egl_state.display, egl_state.surface)
            } == egl::FALSE
            {
                eprintln!("DRM backend unavailable: eglSwapBuffers failed");
                return;
            }

            let next_bo = match unsafe { gbm_surface.lock_front_buffer() } {
                Ok(bo) => bo,
                Err(e) => {
                    eprintln!("DRM backend unavailable: {e}");
                    return;
                }
            };

            let next_fb = match framebuffer_for_bo(&card, &mut framebuffer_cache, &next_bo) {
                Ok(fb) => fb,
                Err(e) => {
                    eprintln!("DRM backend unavailable: {e}");
                    return;
                }
            };

            let mut flip_req = atomic::AtomicModeReq::new();
            if let Err(e) =
                add_plane_properties(&mut flip_req, plane, &plane_props, crtc_handle, next_fb)
            {
                eprintln!("DRM backend unavailable: {e}");
                return;
            }

            if let Err(e) = card.atomic_commit(
                AtomicCommitFlags::NONBLOCK | AtomicCommitFlags::PAGE_FLIP_EVENT,
                flip_req,
            ) {
                eprintln!("DRM backend unavailable: {e}");
                return;
            }

            if let Err(e) = wait_for_page_flip(&card) {
                eprintln!("DRM backend unavailable: {e}");
                return;
            }

            drop(current_bo.take());
            current_bo = Some(next_bo);
        }
        std::thread::sleep(Duration::from_millis(4));
    }
}
