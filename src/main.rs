use std::{ffi::CString, num::NonZeroU32};

use glutin::{
    config::{ConfigTemplateBuilder, GlConfig},
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    prelude::GlSurface,
    surface::{Surface as GlutinSurface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use skia_safe::{
    gpu::{self, backend_render_targets, gl::FramebufferInfo, SurfaceOrigin},
    Color, ColorType, Font, FontMgr, FontStyle, Paint, Rect, Surface,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};

fn create_skia_surface(
    window: &Window,
    fb_info: FramebufferInfo,
    gr_context: &mut skia_safe::gpu::DirectContext,
    num_samples: usize,
    stencil_size: usize,
) -> Surface {
    let size = window.inner_size();
    let (w, h) = (size.width as i32, size.height as i32);

    let backend_render_target =
        backend_render_targets::make_gl((w, h), num_samples, stencil_size, fb_info);

    gpu::surfaces::wrap_backend_render_target(
        gr_context,
        &backend_render_target,
        SurfaceOrigin::BottomLeft,
        ColorType::RGBA8888,
        None,
        None,
    )
    .expect("Could not create Skia surface")
}

struct Env {
    surface: Surface,
    gl_surface: GlutinSurface<WindowSurface>,
    gr_context: skia_safe::gpu::DirectContext,
    gl_context: PossiblyCurrentContext,
    window: Window,
}

struct App {
    env: Env,
    fb_info: FramebufferInfo,
    num_samples: usize,
    stencil_size: usize,
}

impl App {
    fn redraw(&mut self) {
        let canvas = self.env.surface.canvas();
        canvas.clear(Color::WHITE);

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::BLACK);

        let mut p = Paint::default();
        p.set_anti_alias(true);
        p.set_color(Color::from_argb(255, 255, 0, 0)); // red
        canvas.draw_rect(Rect::from_xywh(40.0, 40.0, 200.0, 120.0), &p);

        let fm = FontMgr::new();
        let tf = fm
            .match_family_style("DejaVu Sans", FontStyle::normal())
            .or_else(|| fm.match_family_style("Sans", FontStyle::normal()))
            .expect("No system fonts found");

        let font = Font::new(tf, 48.0);

        // Start without emoji
        canvas.draw_str("Hello, Wayland", (40, 120), &font, &paint);

        self.env.gr_context.flush_and_submit();
        self.env
            .gl_surface
            .swap_buffers(&self.env.gl_context)
            .expect("swap_buffers failed");
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(physical_size) => {
                // Resize GL surface first
                let (w, h): (u32, u32) = physical_size.into();
                self.env.gl_surface.resize(
                    &self.env.gl_context,
                    NonZeroU32::new(w.max(1)).unwrap(),
                    NonZeroU32::new(h.max(1)).unwrap(),
                );

                // Recreate Skia surface for the new size
                self.env.surface = create_skia_surface(
                    &self.env.window,
                    self.fb_info,
                    &mut self.env.gr_context,
                    self.num_samples,
                    self.stencil_size,
                );

                self.env.window.request_redraw();
            }

            WindowEvent::RedrawRequested => self.redraw(),

            _ => {}
        }
    }
}

fn main() {
    // Create winit event loop + window attributes
    let el = EventLoop::new().expect("Failed to create event loop");
    let window_attributes = WindowAttributes::default()
        .with_title("skia-wayland-hello")
        .with_inner_size(LogicalSize::new(800, 600));

    // Ask glutin for a transparent-capable config (optional)
    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(true);

    // Create display + window + choose a config
    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));
    let (window, gl_config) = display_builder
        .build(&el, template, |configs| {
            // pick config with the lowest samples (Skia usually prefers that)
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
        .expect("Failed to build display");

    let window = window.expect("Could not create window");
    let window_handle = window.window_handle().expect("Failed to get window handle");
    let raw_window_handle = window_handle.as_raw();

    // Create GL context (try core GL, fall back to GLES)
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

    // Create the window surface and make context current
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
            .expect("Could not create GL window surface")
    };

    let gl_context = not_current_gl_context
        .make_current(&gl_surface)
        .expect("Could not make GL context current");

    // Load GL function pointers
    gl::load_with(|s| {
        gl_config
            .display()
            .get_proc_address(CString::new(s).unwrap().as_c_str())
    });

    // Create Skia GL interface + DirectContext
    let interface = skia_safe::gpu::gl::Interface::new_load_with(|name| {
        // Workaround used by the upstream example:
        if name == "eglGetCurrentDisplay" {
            return std::ptr::null();
        }
        gl_config
            .display()
            .get_proc_address(CString::new(name).unwrap().as_c_str())
    })
    .expect("Could not create Skia GL interface");

    let mut gr_context =
        skia_safe::gpu::direct_contexts::make_gl(interface, None).expect("make_gl failed");

    // Grab current framebuffer binding for Skia backend render target
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

    let surface = create_skia_surface(&window, fb_info, &mut gr_context, num_samples, stencil_size);

    // IMPORTANT drop order: window must outlive DirectContext (see upstream example notes)
    let env = Env {
        surface,
        gl_surface,
        gr_context,
        gl_context,
        window,
    };

    let mut app = App {
        env,
        fb_info,
        num_samples,
        stencil_size,
    };

    // Kick first frame (Wayland windows often only appear after first draw/present)
    app.env.window.request_redraw();
    el.run_app(&mut app).expect("run_app failed");
}
