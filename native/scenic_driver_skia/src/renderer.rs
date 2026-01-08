use skia_safe::{
    Color, ColorType, Font, FontMgr, FontStyle, Paint, Rect, Surface,
    gpu::{self, SurfaceOrigin, backend_render_targets, gl::FramebufferInfo},
};

fn create_skia_surface(
    dimensions: (i32, i32),
    fb_info: FramebufferInfo,
    gr_context: &mut skia_safe::gpu::DirectContext,
    num_samples: usize,
    stencil_size: usize,
) -> Surface {
    let backend_render_target =
        backend_render_targets::make_gl(dimensions, num_samples, stencil_size, fb_info);

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

#[derive(Clone, Copy)]
pub enum SurfaceSource {
    Gl {
        fb_info: FramebufferInfo,
        num_samples: usize,
        stencil_size: usize,
    },
    Raster,
}

pub struct Renderer {
    surface: Surface,
    gr_context: Option<skia_safe::gpu::DirectContext>,
    source: SurfaceSource,
    text: String,
}

impl Renderer {
    pub fn new(
        dimensions: (u32, u32),
        fb_info: FramebufferInfo,
        gr_context: skia_safe::gpu::DirectContext,
        num_samples: usize,
        stencil_size: usize,
        text: String,
    ) -> Self {
        let mut gr_context = gr_context;
        let surface = create_skia_surface(
            (dimensions.0 as i32, dimensions.1 as i32),
            fb_info,
            &mut gr_context,
            num_samples,
            stencil_size,
        );

        Self {
            surface,
            gr_context: Some(gr_context),
            source: SurfaceSource::Gl {
                fb_info,
                num_samples,
                stencil_size,
            },
            text,
        }
    }

    pub fn from_surface(
        surface: Surface,
        gr_context: Option<skia_safe::gpu::DirectContext>,
        text: String,
    ) -> Self {
        Self {
            surface,
            gr_context,
            source: SurfaceSource::Raster,
            text,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn redraw(&mut self) {
        let canvas = self.surface.canvas();
        canvas.clear(Color::WHITE);

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::BLACK);

        let mut p = Paint::default();
        p.set_anti_alias(true);
        p.set_color(Color::from_argb(255, 255, 0, 0));
        canvas.draw_rect(Rect::from_xywh(40.0, 40.0, 200.0, 120.0), &p);

        let fm = FontMgr::new();
        let tf = fm
            .match_family_style("DejaVu Sans", FontStyle::normal())
            .or_else(|| fm.match_family_style("Sans", FontStyle::normal()))
            .expect("No system fonts found");

        let font = Font::new(tf, 48.0);
        canvas.draw_str(&self.text, (40, 120), &font, &paint);

        if let Some(gr) = self.gr_context.as_mut() {
            gr.flush_and_submit();
        }
    }

    pub fn resize(&mut self, dimensions: (u32, u32)) {
        if let SurfaceSource::Gl {
            fb_info,
            num_samples,
            stencil_size,
        } = self.source
        {
            if let Some(context) = self.gr_context.as_mut() {
                self.surface = create_skia_surface(
                    (dimensions.0 as i32, dimensions.1 as i32),
                    fb_info,
                    context,
                    num_samples,
                    stencil_size,
                );
            }
        }
    }
}
