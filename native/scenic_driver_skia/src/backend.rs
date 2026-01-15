use std::{
    ffi::CString,
    num::NonZeroU32,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::Sender,
    },
};

use glutin::{
    config::{ConfigTemplateBuilder, GlConfig},
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    prelude::GlSurface,
    surface::{Surface as GlutinSurface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use skia_safe::gpu::gl::FramebufferInfo;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::{Key, KeyLocation, ModifiersState, NamedKey},
    platform::wayland::EventLoopBuilderExtWayland,
    window::{Window, WindowAttributes},
};

use crate::input::{
    ACTION_PRESS, ACTION_RELEASE, INPUT_MASK_CODEPOINT, INPUT_MASK_CURSOR_BUTTON,
    INPUT_MASK_CURSOR_POS, INPUT_MASK_CURSOR_SCROLL, INPUT_MASK_KEY, INPUT_MASK_VIEWPORT,
    InputEvent, InputQueue, notify_input_ready,
};
use crate::input_translate::{
    Key as ScenicKey, KeyLocation as ScenicKeyLocation, Modifiers as ScenicModifiers,
    MouseButton as ScenicMouseButton, NamedKey as ScenicNamedKey, button_to_scenic, key_to_scenic,
    modifiers_to_mask,
};
use crate::renderer::{RenderState, Renderer};

#[derive(Debug)]
pub enum UserEvent {
    Stop,
    SetText(String),
    Redraw,
}

struct Env {
    gl_surface: GlutinSurface<WindowSurface>,
    gl_context: PossiblyCurrentContext,
    window: Window,
}

struct App {
    env: Option<Env>,
    renderer: Option<Renderer>,
    running: bool,
    running_flag: Arc<AtomicBool>,
    current_text: String,
    render_state: Arc<Mutex<RenderState>>,
    input_mask: Arc<AtomicU32>,
    input_events: Arc<Mutex<InputQueue>>,
    cursor_pos: (f32, f32),
    window_size: (u32, u32),
    scale_factor: f64,
    modifiers: ModifiersState,
}

impl App {
    fn logical_size(&self, physical: winit::dpi::PhysicalSize<u32>) -> (u32, u32) {
        let logical: LogicalSize<f64> = physical.to_logical(self.scale_factor);
        (logical.width.round() as u32, logical.height.round() as u32)
    }

    fn handle_resize(&mut self, physical_size: winit::dpi::PhysicalSize<u32>) {
        if !self.running {
            return;
        }

        let (w, h): (u32, u32) = physical_size.into();
        if (w, h) != self.window_size {
            self.window_size = (w, h);
            let mask = self.input_mask.load(Ordering::Relaxed);
            if mask & INPUT_MASK_VIEWPORT != 0 {
                let (logical_w, logical_h) = self.logical_size(physical_size);
                self.push_input(InputEvent::ViewportReshape {
                    width: logical_w,
                    height: logical_h,
                });
            }
        }
        if let (Some(env), Some(renderer)) = (self.env.as_mut(), self.renderer.as_mut()) {
            env.gl_surface.resize(
                &env.gl_context,
                NonZeroU32::new(w.max(1)).unwrap(),
                NonZeroU32::new(h.max(1)).unwrap(),
            );

            renderer.resize((w.max(1), h.max(1)));
            env.window.request_redraw();
        }
    }

    fn redraw(&mut self) {
        if let (Some(env), Some(renderer)) = (self.env.as_mut(), self.renderer.as_mut()) {
            if let Ok(render_state) = self.render_state.lock() {
                renderer.set_scale_factor(self.scale_factor as f32);
                renderer.redraw(&render_state);
            }
            env.gl_surface
                .swap_buffers(&env.gl_context)
                .expect("swap_buffers failed");
        }
    }

    fn set_running(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, running: bool) {
        if running && !self.running {
            if self.env.is_none() || self.renderer.is_none() {
                match create_env_renderer_with_active_event_loop(
                    event_loop,
                    self.current_text.clone(),
                ) {
                    Ok((env, renderer)) => {
                        let size = env.window.inner_size();
                        self.env = Some(env);
                        self.renderer = Some(renderer);
                        self.window_size = (size.width, size.height);
                        if let Some(env) = self.env.as_ref() {
                            self.scale_factor = env.window.scale_factor();
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to initialize renderer: {err}");
                        self.running_flag.store(false, Ordering::Relaxed);
                        return;
                    }
                }
            }
        } else if !running && self.running {
            if let Some(env) = self.env.as_ref() {
                env.window.set_visible(false);
            }
            self.renderer = None;
            self.env = None;
        }

        self.running = running;
        self.running_flag.store(running, Ordering::Relaxed);
        if running && let Some(env) = self.env.as_ref() {
            env.window.request_redraw();
        }
    }

    fn push_input(&self, event: InputEvent) {
        let notify = if let Ok(mut queue) = self.input_events.lock() {
            queue.push_event(event)
        } else {
            None
        };

        if let Some(pid) = notify {
            notify_input_ready(pid);
        }
    }
}

#[derive(Clone, Debug)]
pub struct WaylandWindowConfig {
    pub requested_size: Option<(u32, u32)>,
    pub window_title: String,
    pub window_resizeable: bool,
}

fn create_env_renderer_with_event_loop(
    event_loop: &EventLoop<UserEvent>,
    initial_text: String,
    config: WaylandWindowConfig,
) -> Result<(Env, Renderer), String> {
    let window_attributes = WindowAttributes::default()
        .with_title(config.window_title)
        .with_resizable(config.window_resizeable);
    let window_attributes = if let Some((width, height)) = config.requested_size {
        window_attributes.with_inner_size(LogicalSize::new(width, height))
    } else {
        window_attributes.with_inner_size(LogicalSize::new(800, 600))
    };

    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(true);

    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));
    let (window, gl_config) = display_builder
        .build(event_loop, template, |configs| {
            configs
                .reduce(|accum, cfg| {
                    if cfg.num_samples() < accum.num_samples() {
                        cfg
                    } else {
                        accum
                    }
                })
                .unwrap()
        })
        .map_err(|err| format!("failed to build display: {err}"))?;

    let window = window.ok_or_else(|| "could not create window".to_string())?;
    let window_handle = window
        .window_handle()
        .map_err(|err| format!("failed to get window handle: {err}"))?;
    let raw_window_handle = window_handle.as_raw();

    let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(Some(raw_window_handle));

    let not_current_gl_context = unsafe {
        gl_config
            .display()
            .create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_config
                    .display()
                    .create_context(&gl_config, &fallback_context_attributes)
                    .expect("failed to create GL/GLES context")
            })
    };

    let (width, height): (u32, u32) = window.inner_size().into();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        raw_window_handle,
        NonZeroU32::new(width.max(1)).unwrap(),
        NonZeroU32::new(height.max(1)).unwrap(),
    );

    let gl_surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attrs)
            .map_err(|err| format!("could not create GL window surface: {err}"))?
    };

    let gl_context = not_current_gl_context
        .make_current(&gl_surface)
        .map_err(|err| format!("could not make GL context current: {err}"))?;

    gl::load_with(|s| {
        gl_config
            .display()
            .get_proc_address(CString::new(s).unwrap().as_c_str())
    });

    let interface = skia_safe::gpu::gl::Interface::new_load_with(|name| {
        if name == "eglGetCurrentDisplay" {
            return std::ptr::null();
        }
        gl_config
            .display()
            .get_proc_address(CString::new(name).unwrap().as_c_str())
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

    let num_samples = gl_config.num_samples() as usize;
    let stencil_size = gl_config.stencil_size() as usize;

    let renderer = Renderer::new(
        (width, height),
        fb_info,
        gr_context,
        num_samples,
        stencil_size,
        initial_text,
    );

    let env = Env {
        gl_surface,
        gl_context,
        window,
    };

    Ok((env, renderer))
}

fn create_env_renderer_with_active_event_loop(
    event_loop: &winit::event_loop::ActiveEventLoop,
    initial_text: String,
) -> Result<(Env, Renderer), String> {
    let window_attributes = WindowAttributes::default()
        .with_title("skia-wayland-hello")
        .with_inner_size(LogicalSize::new(800, 600));

    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(true);

    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));
    let (window, gl_config) = display_builder
        .build(event_loop, template, |configs| {
            configs
                .reduce(|accum, cfg| {
                    if cfg.num_samples() < accum.num_samples() {
                        cfg
                    } else {
                        accum
                    }
                })
                .unwrap()
        })
        .map_err(|err| format!("failed to build display: {err}"))?;

    let window = window.ok_or_else(|| "could not create window".to_string())?;
    let window_handle = window
        .window_handle()
        .map_err(|err| format!("failed to get window handle: {err}"))?;
    let raw_window_handle = window_handle.as_raw();

    let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(Some(raw_window_handle));

    let not_current_gl_context = unsafe {
        gl_config
            .display()
            .create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_config
                    .display()
                    .create_context(&gl_config, &fallback_context_attributes)
                    .expect("failed to create GL/GLES context")
            })
    };

    let (width, height): (u32, u32) = window.inner_size().into();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        raw_window_handle,
        NonZeroU32::new(width.max(1)).unwrap(),
        NonZeroU32::new(height.max(1)).unwrap(),
    );

    let gl_surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attrs)
            .map_err(|err| format!("could not create GL window surface: {err}"))?
    };

    let gl_context = not_current_gl_context
        .make_current(&gl_surface)
        .map_err(|err| format!("could not make GL context current: {err}"))?;

    gl::load_with(|s| {
        gl_config
            .display()
            .get_proc_address(CString::new(s).unwrap().as_c_str())
    });

    let interface = skia_safe::gpu::gl::Interface::new_load_with(|name| {
        if name == "eglGetCurrentDisplay" {
            return std::ptr::null();
        }
        gl_config
            .display()
            .get_proc_address(CString::new(name).unwrap().as_c_str())
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

    let num_samples = gl_config.num_samples() as usize;
    let stencil_size = gl_config.stencil_size() as usize;

    let renderer = Renderer::new(
        (width, height),
        fb_info,
        gr_context,
        num_samples,
        stencil_size,
        initial_text,
    );

    let env = Env {
        gl_surface,
        gl_context,
        window,
    };

    Ok((env, renderer))
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let mask = self.input_mask.load(Ordering::Relaxed);
                if mask & INPUT_MASK_KEY != 0 {
                    let action = match event.state {
                        ElementState::Pressed => ACTION_PRESS,
                        ElementState::Released => ACTION_RELEASE,
                    };
                    let key = key_to_scenic(
                        map_key(&event.logical_key),
                        map_key_location(event.location),
                    );
                    let mods = modifiers_to_mask(map_modifiers(self.modifiers));
                    self.push_input(InputEvent::Key { key, action, mods });
                }

                if mask & INPUT_MASK_CODEPOINT != 0
                    && matches!(event.state, ElementState::Pressed)
                    && let Some(text) = event.text.as_ref()
                {
                    let mods = modifiers_to_mask(map_modifiers(self.modifiers));
                    for ch in text.chars() {
                        self.push_input(InputEvent::Codepoint {
                            codepoint: ch,
                            mods,
                        });
                    }
                }
            }

            WindowEvent::Ime(ime) => {
                let mask = self.input_mask.load(Ordering::Relaxed);
                if mask & INPUT_MASK_CODEPOINT != 0
                    && let winit::event::Ime::Commit(text) = ime
                {
                    let mods = modifiers_to_mask(map_modifiers(self.modifiers));
                    for ch in text.chars() {
                        self.push_input(InputEvent::Codepoint {
                            codepoint: ch,
                            mods,
                        });
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let mask = self.input_mask.load(Ordering::Relaxed);
                let logical: LogicalPosition<f64> = position.to_logical(self.scale_factor);
                let x = logical.x as f32;
                let y = logical.y as f32;
                self.cursor_pos = (x, y);
                if mask & INPUT_MASK_CURSOR_POS != 0 {
                    self.push_input(InputEvent::CursorPos { x, y });
                }
            }

            WindowEvent::CursorEntered { .. } => {
                let mask = self.input_mask.load(Ordering::Relaxed);
                if mask & INPUT_MASK_VIEWPORT != 0 {
                    let (x, y) = self.cursor_pos;
                    self.push_input(InputEvent::Viewport {
                        entered: true,
                        x,
                        y,
                    });
                }
            }

            WindowEvent::CursorLeft { .. } => {
                let mask = self.input_mask.load(Ordering::Relaxed);
                if mask & INPUT_MASK_VIEWPORT != 0 {
                    let (x, y) = self.cursor_pos;
                    self.push_input(InputEvent::Viewport {
                        entered: false,
                        x,
                        y,
                    });
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let mask = self.input_mask.load(Ordering::Relaxed);
                if mask & INPUT_MASK_CURSOR_BUTTON != 0 {
                    let action = match state {
                        ElementState::Pressed => ACTION_PRESS,
                        ElementState::Released => ACTION_RELEASE,
                    };
                    let button = button_to_scenic(map_mouse_button(button));
                    let mods = modifiers_to_mask(map_modifiers(self.modifiers));
                    let (x, y) = self.cursor_pos;
                    self.push_input(InputEvent::CursorButton {
                        button,
                        action,
                        mods,
                        x,
                        y,
                    });
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let mask = self.input_mask.load(Ordering::Relaxed);
                if mask & INPUT_MASK_CURSOR_SCROLL != 0 {
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x, y),
                        MouseScrollDelta::PixelDelta(pos) => {
                            let logical: LogicalPosition<f64> = pos.to_logical(self.scale_factor);
                            (logical.x as f32, logical.y as f32)
                        }
                    };
                    let (x, y) = self.cursor_pos;
                    self.push_input(InputEvent::CursorScroll { dx, dy, x, y });
                }
            }

            WindowEvent::CloseRequested => self.set_running(_event_loop, false),

            WindowEvent::Resized(physical_size) => {
                self.handle_resize(physical_size);
            }

            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => {
                self.scale_factor = scale_factor;
                if let Some(env) = self.env.as_ref() {
                    self.handle_resize(env.window.inner_size());
                }
            }

            WindowEvent::RedrawRequested => {
                if self.running {
                    self.redraw();
                }
            }

            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Stop => self.set_running(event_loop, false),
            UserEvent::SetText(text) => {
                self.current_text = text.clone();
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.set_text(text);
                }
                if self.running {
                    self.redraw();
                }
            }
            UserEvent::Redraw => {
                if self.running {
                    self.redraw();
                }
            }
        }
    }
}

pub fn run(
    proxy_ready: Sender<EventLoopProxy<UserEvent>>,
    initial_text: String,
    running_flag: Arc<AtomicBool>,
    render_state: Arc<Mutex<RenderState>>,
    input_mask: Arc<AtomicU32>,
    input_events: Arc<Mutex<InputQueue>>,
    config: WaylandWindowConfig,
) {
    let mut el_builder = EventLoop::<UserEvent>::with_user_event();
    EventLoopBuilderExtWayland::with_any_thread(&mut el_builder, true);
    let el = el_builder.build().expect("Failed to create event loop");
    let proxy = el.create_proxy();
    let _ = proxy_ready.send(proxy);
    let (env, renderer) =
        match create_env_renderer_with_event_loop(&el, initial_text.clone(), config) {
            Ok(values) => values,
            Err(err) => {
                eprintln!("Failed to initialize renderer: {err}");
                running_flag.store(false, Ordering::Relaxed);
                return;
            }
        };
    let size = env.window.inner_size();
    let scale_factor = env.window.scale_factor();

    let mut app = App {
        env: Some(env),
        renderer: Some(renderer),
        running: true,
        running_flag,
        current_text: initial_text,
        render_state,
        input_mask,
        input_events,
        cursor_pos: (0.0, 0.0),
        window_size: (size.width, size.height),
        scale_factor,
        modifiers: ModifiersState::empty(),
    };
    app.redraw();
    el.run_app(&mut app).expect("run_app failed");
}

fn map_modifiers(mods: ModifiersState) -> ScenicModifiers {
    ScenicModifiers {
        shift: mods.shift_key(),
        ctrl: mods.control_key(),
        alt: mods.alt_key(),
        meta: mods.super_key(),
    }
}

fn map_key_location(location: KeyLocation) -> ScenicKeyLocation {
    match location {
        KeyLocation::Left => ScenicKeyLocation::Left,
        KeyLocation::Right => ScenicKeyLocation::Right,
        KeyLocation::Numpad => ScenicKeyLocation::Numpad,
        KeyLocation::Standard => ScenicKeyLocation::Standard,
    }
}

fn map_key(key: &Key) -> ScenicKey {
    match key {
        Key::Character(text) => text
            .chars()
            .next()
            .map(ScenicKey::Character)
            .unwrap_or(ScenicKey::Unidentified),
        Key::Named(named) => map_named_key(*named)
            .map(ScenicKey::Named)
            .unwrap_or(ScenicKey::Unidentified),
        Key::Unidentified(_) | Key::Dead(_) => ScenicKey::Unidentified,
    }
}

fn map_named_key(named: NamedKey) -> Option<ScenicNamedKey> {
    Some(match named {
        NamedKey::Enter => ScenicNamedKey::Enter,
        NamedKey::Tab => ScenicNamedKey::Tab,
        NamedKey::Space => ScenicNamedKey::Space,
        NamedKey::Escape => ScenicNamedKey::Escape,
        NamedKey::Backspace => ScenicNamedKey::Backspace,
        NamedKey::Insert => ScenicNamedKey::Insert,
        NamedKey::Delete => ScenicNamedKey::Delete,
        NamedKey::ArrowLeft => ScenicNamedKey::ArrowLeft,
        NamedKey::ArrowRight => ScenicNamedKey::ArrowRight,
        NamedKey::ArrowUp => ScenicNamedKey::ArrowUp,
        NamedKey::ArrowDown => ScenicNamedKey::ArrowDown,
        NamedKey::PageUp => ScenicNamedKey::PageUp,
        NamedKey::PageDown => ScenicNamedKey::PageDown,
        NamedKey::Home => ScenicNamedKey::Home,
        NamedKey::End => ScenicNamedKey::End,
        NamedKey::CapsLock => ScenicNamedKey::CapsLock,
        NamedKey::ScrollLock => ScenicNamedKey::ScrollLock,
        NamedKey::NumLock => ScenicNamedKey::NumLock,
        NamedKey::PrintScreen => ScenicNamedKey::PrintScreen,
        NamedKey::Pause => ScenicNamedKey::Pause,
        NamedKey::ContextMenu => ScenicNamedKey::ContextMenu,
        NamedKey::Shift => ScenicNamedKey::Shift,
        NamedKey::Control => ScenicNamedKey::Control,
        NamedKey::Alt => ScenicNamedKey::Alt,
        NamedKey::AltGraph => ScenicNamedKey::AltGraph,
        NamedKey::Super => ScenicNamedKey::Super,
        NamedKey::Meta => ScenicNamedKey::Meta,
        NamedKey::Hyper => ScenicNamedKey::Hyper,
        NamedKey::F1 => ScenicNamedKey::F1,
        NamedKey::F2 => ScenicNamedKey::F2,
        NamedKey::F3 => ScenicNamedKey::F3,
        NamedKey::F4 => ScenicNamedKey::F4,
        NamedKey::F5 => ScenicNamedKey::F5,
        NamedKey::F6 => ScenicNamedKey::F6,
        NamedKey::F7 => ScenicNamedKey::F7,
        NamedKey::F8 => ScenicNamedKey::F8,
        NamedKey::F9 => ScenicNamedKey::F9,
        NamedKey::F10 => ScenicNamedKey::F10,
        NamedKey::F11 => ScenicNamedKey::F11,
        NamedKey::F12 => ScenicNamedKey::F12,
        NamedKey::F13 => ScenicNamedKey::F13,
        NamedKey::F14 => ScenicNamedKey::F14,
        NamedKey::F15 => ScenicNamedKey::F15,
        NamedKey::F16 => ScenicNamedKey::F16,
        NamedKey::F17 => ScenicNamedKey::F17,
        NamedKey::F18 => ScenicNamedKey::F18,
        NamedKey::F19 => ScenicNamedKey::F19,
        NamedKey::F20 => ScenicNamedKey::F20,
        NamedKey::F21 => ScenicNamedKey::F21,
        NamedKey::F22 => ScenicNamedKey::F22,
        NamedKey::F23 => ScenicNamedKey::F23,
        NamedKey::F24 => ScenicNamedKey::F24,
        _ => return None,
    })
}

fn map_mouse_button(button: winit::event::MouseButton) -> ScenicMouseButton {
    match button {
        winit::event::MouseButton::Left => ScenicMouseButton::Left,
        winit::event::MouseButton::Right => ScenicMouseButton::Right,
        winit::event::MouseButton::Middle => ScenicMouseButton::Middle,
        winit::event::MouseButton::Back => ScenicMouseButton::Back,
        winit::event::MouseButton::Forward => ScenicMouseButton::Forward,
        winit::event::MouseButton::Other(_) => ScenicMouseButton::Other,
    }
}
