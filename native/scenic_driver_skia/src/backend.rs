use std::{
    ffi::CString,
    num::NonZeroU32,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
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
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{EventLoop, EventLoopProxy},
    platform::wayland::EventLoopBuilderExtWayland,
    window::{Window, WindowAttributes},
};

use crate::renderer::{RenderState, Renderer};

#[derive(Debug)]
pub enum UserEvent {
    Stop,
    SetText(String),
    Start,
    SetRenderState(RenderState),
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
    render_state: RenderState,
}

impl App {
    fn redraw(&mut self) {
        if let (Some(env), Some(renderer)) = (self.env.as_mut(), self.renderer.as_mut()) {
            renderer.set_state(self.render_state.clone());
            renderer.redraw();
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
                    self.render_state.clone(),
                ) {
                    Ok((env, renderer)) => {
                        self.env = Some(env);
                        self.renderer = Some(renderer);
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
        if running {
            if let Some(env) = self.env.as_ref() {
                env.window.request_redraw();
            }
        }
    }
}

fn create_env_renderer_with_event_loop(
    event_loop: &EventLoop<UserEvent>,
    initial_text: String,
    render_state: RenderState,
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
        render_state,
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
    render_state: RenderState,
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
        render_state,
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
            WindowEvent::CloseRequested => self.set_running(_event_loop, false),

            WindowEvent::Resized(physical_size) => {
                if !self.running {
                    return;
                }
                let (w, h): (u32, u32) = physical_size.into();
                if let (Some(env), Some(renderer)) = (self.env.as_mut(), self.renderer.as_mut()) {
                    env.gl_surface.resize(
                        &env.gl_context,
                        NonZeroU32::new(w.max(1)).unwrap(),
                        NonZeroU32::new(h.max(1)).unwrap(),
                    );

                    renderer.resize((w, h));
                    env.window.request_redraw();
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
            UserEvent::Start => self.set_running(event_loop, true),
            UserEvent::SetText(text) => {
                self.current_text = text.clone();
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.set_text(text);
                }
                if self.running {
                    self.redraw();
                }
            }
            UserEvent::SetRenderState(render_state) => {
                self.render_state = render_state;
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
    render_state: RenderState,
) {
    let mut el_builder = EventLoop::<UserEvent>::with_user_event();
    EventLoopBuilderExtWayland::with_any_thread(&mut el_builder, true);
    let el = el_builder.build().expect("Failed to create event loop");
    let proxy = el.create_proxy();
    let _ = proxy_ready.send(proxy);
    let (env, renderer) = match create_env_renderer_with_event_loop(
        &el,
        initial_text.clone(),
        render_state.clone(),
    ) {
        Ok(values) => values,
        Err(err) => {
            eprintln!("Failed to initialize renderer: {err}");
            running_flag.store(false, Ordering::Relaxed);
            return;
        }
    };

    let mut app = App {
        env: Some(env),
        renderer: Some(renderer),
        running: true,
        running_flag,
        current_text: initial_text,
        render_state,
    };
    app.redraw();
    el.run_app(&mut app).expect("run_app failed");
}
