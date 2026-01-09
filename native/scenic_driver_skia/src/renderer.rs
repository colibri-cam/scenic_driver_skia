use std::collections::HashMap;

use skia_safe::{
    Color, ColorType, Font, FontMgr, FontStyle, Paint, Rect, Surface,
    gpu::{self, SurfaceOrigin, backend_render_targets, gl::FramebufferInfo},
};

#[derive(Clone, Debug, PartialEq)]
pub enum ScriptOp {
    PushState,
    PopState,
    PopPushState,
    Translate(f32, f32),
    FillColor(Color),
    DrawRect { width: f32, height: f32, flag: u16 },
    DrawText(String),
    DrawScript(String),
}

#[derive(Clone, Debug)]
pub struct RenderState {
    pub clear_color: Color,
    pub scripts: HashMap<String, Vec<ScriptOp>>,
    pub root_id: Option<String>,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            clear_color: Color::WHITE,
            scripts: HashMap::new(),
            root_id: None,
        }
    }
}

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
    render_state: RenderState,
}

impl Renderer {
    pub fn new(
        dimensions: (u32, u32),
        fb_info: FramebufferInfo,
        gr_context: skia_safe::gpu::DirectContext,
        num_samples: usize,
        stencil_size: usize,
        text: String,
        render_state: RenderState,
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
            render_state,
        }
    }

    pub fn from_surface(
        surface: Surface,
        gr_context: Option<skia_safe::gpu::DirectContext>,
        text: String,
        render_state: RenderState,
    ) -> Self {
        Self {
            surface,
            gr_context,
            source: SurfaceSource::Raster,
            text,
            render_state,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn set_state(&mut self, render_state: RenderState) {
        self.render_state = render_state;
    }

    pub fn surface_mut(&mut self) -> &mut Surface {
        &mut self.surface
    }

    pub fn redraw(&mut self) {
        let render_state = self.render_state.clone();
        let canvas = self.surface.canvas();
        canvas.clear(render_state.clear_color);

        let font = default_font();
        if let Some(root_id) = render_state.root_id.clone() {
            let mut draw_state = DrawState::default();
            let mut stack_ids = Vec::new();
            draw_script(
                &render_state,
                &root_id,
                canvas,
                &mut draw_state,
                &mut stack_ids,
                font.as_ref(),
            );
        }

        if !self.text.is_empty() {
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(Color::BLACK);
            if let Some(font) = font.as_ref() {
                canvas.draw_str(&self.text, (40, 120), font, &paint);
            }
        }

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

fn draw_script(
    render_state: &RenderState,
    script_id: &str,
    canvas: &skia_safe::Canvas,
    draw_state: &mut DrawState,
    stack_ids: &mut Vec<String>,
    font: Option<&Font>,
) {
    if stack_ids.iter().any(|id| id == script_id) {
        return;
    }

    let ops = match render_state.scripts.get(script_id) {
        Some(ops) => ops,
        None => return,
    };

    stack_ids.push(script_id.to_string());

    for op in ops {
        match op {
            ScriptOp::PushState => draw_state.push(),
            ScriptOp::PopState => draw_state.pop(),
            ScriptOp::PopPushState => draw_state.pop_push(),
            ScriptOp::Translate(x, y) => draw_state.translate = (*x, *y),
            ScriptOp::FillColor(color) => draw_state.fill_color = *color,
            ScriptOp::DrawRect {
                width,
                height,
                flag,
            } => {
                if flag & 0x01 == 0x01 {
                    let rect = Rect::from_xywh(
                        draw_state.translate.0,
                        draw_state.translate.1,
                        *width,
                        *height,
                    );
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(draw_state.fill_color);
                    canvas.draw_rect(rect, &paint);
                }
            }
            ScriptOp::DrawText(text) => {
                if let Some(font) = font {
                    if !text.is_empty() {
                        let mut paint = Paint::default();
                        paint.set_anti_alias(true);
                        paint.set_color(draw_state.fill_color);
                        canvas.draw_str(text, draw_state.translate, font, &paint);
                    }
                }
            }
            ScriptOp::DrawScript(id) => {
                draw_script(render_state, id, canvas, draw_state, stack_ids, font);
            }
        }
    }

    stack_ids.pop();
}

fn default_font() -> Option<Font> {
    let fm = FontMgr::new();
    let tf = fm
        .match_family_style("DejaVu Sans", FontStyle::normal())
        .or_else(|| fm.match_family_style("Sans", FontStyle::normal()))?;
    Some(Font::new(tf, 48.0))
}

#[derive(Clone)]
struct DrawState {
    translate: (f32, f32),
    fill_color: Color,
    stack: Vec<((f32, f32), Color)>,
}

impl Default for DrawState {
    fn default() -> Self {
        Self {
            translate: (0.0, 0.0),
            fill_color: Color::RED,
            stack: Vec::new(),
        }
    }
}

impl DrawState {
    fn push(&mut self) {
        self.stack.push((self.translate, self.fill_color));
    }

    fn pop(&mut self) {
        if let Some((translate, fill_color)) = self.stack.pop() {
            self.translate = translate;
            self.fill_color = fill_color;
        } else {
            self.translate = (0.0, 0.0);
            self.fill_color = Color::RED;
        }
    }

    fn pop_push(&mut self) {
        let current = self.stack.pop().unwrap_or(((0.0, 0.0), Color::RED));
        self.translate = current.0;
        self.fill_color = current.1;
        self.stack.push(current);
    }
}
