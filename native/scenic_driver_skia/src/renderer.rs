use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use skia_safe::{
    Color, ColorType, Font, FontMgr, FontStyle, Matrix, Paint, PaintStyle, PathBuilder, Point,
    RRect, Rect, Surface, Typeface, Vector,
    gpu::{self, SurfaceOrigin, backend_render_targets, gl::FramebufferInfo},
};

#[derive(Clone, Debug, PartialEq)]
pub enum ScriptOp {
    PushState,
    PopState,
    PopPushState,
    Translate(f32, f32),
    Rotate(f32),
    Scale(f32, f32),
    Transform {
        a: f32,
        b: f32,
        c: f32,
        d: f32,
        e: f32,
        f: f32,
    },
    FillColor(Color),
    StrokeColor(Color),
    StrokeWidth(f32),
    DrawLine {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        flag: u16,
    },
    DrawCircle {
        radius: f32,
        flag: u16,
    },
    DrawEllipse {
        radius0: f32,
        radius1: f32,
        flag: u16,
    },
    DrawArc {
        radius: f32,
        radians: f32,
        flag: u16,
    },
    DrawSector {
        radius: f32,
        radians: f32,
        flag: u16,
    },
    DrawRect {
        width: f32,
        height: f32,
        flag: u16,
    },
    DrawRRect {
        width: f32,
        height: f32,
        radius: f32,
        flag: u16,
    },
    DrawRRectV {
        width: f32,
        height: f32,
        ul_radius: f32,
        ur_radius: f32,
        lr_radius: f32,
        ll_radius: f32,
        flag: u16,
    },
    DrawText(String),
    Font(String),
    FontSize(f32),
    TextAlign(TextAlign),
    TextBase(TextBase),
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

    pub fn surface_mut(&mut self) -> &mut Surface {
        &mut self.surface
    }

    pub fn redraw(&mut self, render_state: &RenderState) {
        let canvas = self.surface.canvas();
        canvas.clear(render_state.clear_color);

        if let Some(root_id) = render_state.root_id.clone() {
            let mut draw_state = DrawState::default();
            let mut stack_ids = Vec::new();
            draw_script(
                render_state,
                &root_id,
                canvas,
                &mut draw_state,
                &mut stack_ids,
            );
        }

        if !self.text.is_empty() {
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(Color::BLACK);
            if let Some(font) = default_font(DrawState::DEFAULT_FONT_SIZE).as_ref() {
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
            && let Some(context) = self.gr_context.as_mut()
        {
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

fn draw_script(
    render_state: &RenderState,
    script_id: &str,
    canvas: &skia_safe::Canvas,
    draw_state: &mut DrawState,
    stack_ids: &mut Vec<String>,
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
            ScriptOp::PushState => {
                canvas.save();
                draw_state.push();
            }
            ScriptOp::PopState => {
                if draw_state.can_pop() {
                    canvas.restore();
                    draw_state.pop();
                }
            }
            ScriptOp::PopPushState => {
                if draw_state.can_pop() {
                    canvas.restore();
                    canvas.save();
                    draw_state.pop_push();
                }
            }
            ScriptOp::Translate(x, y) => {
                canvas.translate(Vector::new(*x, *y));
            }
            ScriptOp::Rotate(radians) => {
                canvas.rotate(radians.to_degrees(), None);
            }
            ScriptOp::Scale(x, y) => {
                canvas.scale((*x, *y));
            }
            ScriptOp::Transform { a, b, c, d, e, f } => {
                let matrix = Matrix::new_all(*a, *c, *e, *b, *d, *f, 0.0, 0.0, 1.0);
                canvas.concat(&matrix);
            }
            ScriptOp::FillColor(color) => draw_state.fill_color = *color,
            ScriptOp::StrokeColor(color) => draw_state.stroke_color = *color,
            ScriptOp::StrokeWidth(width) => draw_state.stroke_width = *width,
            ScriptOp::DrawLine {
                x0,
                y0,
                x1,
                y1,
                flag,
            } => {
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_color(draw_state.stroke_color);
                    paint.set_stroke_width(draw_state.stroke_width);
                    canvas.draw_line(Point::new(*x0, *y0), Point::new(*x1, *y1), &paint);
                }
            }
            ScriptOp::DrawCircle { radius, flag } => {
                if flag & 0x01 == 0x01 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(draw_state.fill_color);
                    canvas.draw_circle(Point::new(0.0, 0.0), *radius, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_color(draw_state.stroke_color);
                    paint.set_stroke_width(draw_state.stroke_width);
                    canvas.draw_circle(Point::new(0.0, 0.0), *radius, &paint);
                }
            }
            ScriptOp::DrawEllipse {
                radius0,
                radius1,
                flag,
            } => {
                let rect = Rect::from_xywh(-radius0, -radius1, radius0 * 2.0, radius1 * 2.0);
                if flag & 0x01 == 0x01 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(draw_state.fill_color);
                    canvas.draw_oval(rect, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_color(draw_state.stroke_color);
                    paint.set_stroke_width(draw_state.stroke_width);
                    canvas.draw_oval(rect, &paint);
                }
            }
            ScriptOp::DrawArc {
                radius,
                radians,
                flag,
            } => {
                let rect = Rect::from_xywh(-radius, -radius, radius * 2.0, radius * 2.0);
                let start = 0.0;
                let sweep = radians.to_degrees();
                if flag & 0x01 == 0x01 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(draw_state.fill_color);
                    canvas.draw_arc(rect, start, sweep, false, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_color(draw_state.stroke_color);
                    paint.set_stroke_width(draw_state.stroke_width);
                    canvas.draw_arc(rect, start, sweep, false, &paint);
                }
            }
            ScriptOp::DrawSector {
                radius,
                radians,
                flag,
            } => {
                let rect = Rect::from_xywh(-radius, -radius, radius * 2.0, radius * 2.0);
                let sweep = radians.to_degrees();
                let mut builder = PathBuilder::new();
                builder
                    .move_to(Point::new(0.0, 0.0))
                    .line_to(Point::new(*radius, 0.0))
                    .arc_to(rect, 0.0, sweep, false)
                    .close();
                let path = builder.detach();
                if flag & 0x01 == 0x01 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(draw_state.fill_color);
                    canvas.draw_path(&path, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_color(draw_state.stroke_color);
                    paint.set_stroke_width(draw_state.stroke_width);
                    canvas.draw_path(&path, &paint);
                }
            }
            ScriptOp::DrawRect {
                width,
                height,
                flag,
            } => {
                if flag & 0x01 == 0x01 {
                    let rect = Rect::from_xywh(0.0, 0.0, *width, *height);
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(draw_state.fill_color);
                    canvas.draw_rect(rect, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let rect = Rect::from_xywh(0.0, 0.0, *width, *height);
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_color(draw_state.stroke_color);
                    paint.set_stroke_width(draw_state.stroke_width);
                    canvas.draw_rect(rect, &paint);
                }
            }
            ScriptOp::DrawRRect {
                width,
                height,
                radius,
                flag,
            } => {
                let rect = Rect::from_xywh(0.0, 0.0, *width, *height);
                let rrect = RRect::new_rect_xy(rect, *radius, *radius);
                if flag & 0x01 == 0x01 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(draw_state.fill_color);
                    canvas.draw_rrect(rrect, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_color(draw_state.stroke_color);
                    paint.set_stroke_width(draw_state.stroke_width);
                    canvas.draw_rrect(rrect, &paint);
                }
            }
            ScriptOp::DrawRRectV {
                width,
                height,
                ul_radius,
                ur_radius,
                lr_radius,
                ll_radius,
                flag,
            } => {
                let rect = Rect::from_xywh(0.0, 0.0, *width, *height);
                let radii = [
                    Vector::new(*ul_radius, *ul_radius),
                    Vector::new(*ur_radius, *ur_radius),
                    Vector::new(*lr_radius, *lr_radius),
                    Vector::new(*ll_radius, *ll_radius),
                ];
                let rrect = RRect::new_rect_radii(rect, &radii);
                if flag & 0x01 == 0x01 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(draw_state.fill_color);
                    canvas.draw_rrect(rrect, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_color(draw_state.stroke_color);
                    paint.set_stroke_width(draw_state.stroke_width);
                    canvas.draw_rrect(rrect, &paint);
                }
            }
            ScriptOp::DrawText(text) => {
                let font = match draw_state.font_id.as_deref() {
                    Some(font_id) => font_from_asset(font_id, draw_state.font_size),
                    None => default_font(draw_state.font_size),
                };
                if let Some(font) = font.as_ref()
                    && !text.is_empty()
                {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(draw_state.fill_color);
                    let (dx, dy) = draw_state.text_offsets(text, font, &paint);
                    canvas.draw_str(text, (dx, dy), font, &paint);
                }
            }
            ScriptOp::Font(font_id) => draw_state.font_id = Some(font_id.clone()),
            ScriptOp::FontSize(size) => draw_state.font_size = *size,
            ScriptOp::TextAlign(align) => draw_state.text_align = *align,
            ScriptOp::TextBase(base) => draw_state.text_base = *base,
            ScriptOp::DrawScript(id) => {
                draw_script(render_state, id, canvas, draw_state, stack_ids);
            }
        }
    }

    stack_ids.pop();
}

fn default_font(size: f32) -> Option<Font> {
    static DEFAULT_TYPEFACE: OnceLock<Option<Typeface>> = OnceLock::new();
    let typeface = DEFAULT_TYPEFACE
        .get_or_init(|| {
            let fm = FontMgr::new();
            fm.match_family_style("DejaVu Sans", FontStyle::normal())
                .or_else(|| fm.match_family_style("Sans", FontStyle::normal()))
        })
        .clone()?;
    Some(Font::new(typeface, size))
}

fn font_from_asset(font_id: &str, size: f32) -> Option<Font> {
    let typeface = typeface_from_asset(font_id)?;
    Some(Font::new(typeface, size))
}

fn typeface_from_asset(font_id: &str) -> Option<Typeface> {
    static FONT_CACHE: OnceLock<Mutex<HashMap<String, Typeface>>> = OnceLock::new();
    let cache = FONT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(cache) = cache.lock()
        && let Some(typeface) = cache.get(font_id)
    {
        return Some(typeface.clone());
    }

    let mut path = std::env::current_dir().ok()?;
    path.push("priv");
    path.push("__scenic");
    path.push("assets");
    path.push(font_id);
    let bytes = std::fs::read(path).ok()?;
    let fm = FontMgr::new();
    let typeface = fm.new_from_data(&bytes, 0)?;

    if let Ok(mut cache) = cache.lock() {
        cache.insert(font_id.to_string(), typeface.clone());
    }

    Some(typeface)
}

#[derive(Clone)]
struct DrawState {
    fill_color: Color,
    stroke_color: Color,
    stroke_width: f32,
    font_id: Option<String>,
    font_size: f32,
    text_align: TextAlign,
    text_base: TextBase,
    stack: Vec<DrawStateSnapshot>,
}

impl Default for DrawState {
    fn default() -> Self {
        Self {
            fill_color: Color::BLACK,
            stroke_color: Color::BLACK,
            stroke_width: 1.0,
            font_id: None,
            font_size: Self::DEFAULT_FONT_SIZE,
            text_align: TextAlign::Left,
            text_base: TextBase::Alphabetic,
            stack: Vec::new(),
        }
    }
}

impl DrawState {
    const DEFAULT_FONT_SIZE: f32 = 20.0;

    fn push(&mut self) {
        self.stack.push(DrawStateSnapshot {
            fill_color: self.fill_color,
            stroke_color: self.stroke_color,
            stroke_width: self.stroke_width,
            font_id: self.font_id.clone(),
            font_size: self.font_size,
            text_align: self.text_align,
            text_base: self.text_base,
        });
    }

    fn pop(&mut self) {
        let snapshot = self.stack.pop().unwrap_or_default();
        self.apply_snapshot(snapshot);
    }

    fn pop_push(&mut self) {
        let snapshot = self.stack.pop().unwrap_or_default();
        self.apply_snapshot(snapshot.clone());
        self.stack.push(snapshot);
    }

    fn can_pop(&self) -> bool {
        !self.stack.is_empty()
    }

    fn apply_snapshot(&mut self, snapshot: DrawStateSnapshot) {
        self.fill_color = snapshot.fill_color;
        self.stroke_color = snapshot.stroke_color;
        self.stroke_width = snapshot.stroke_width;
        self.font_id = snapshot.font_id;
        self.font_size = snapshot.font_size;
        self.text_align = snapshot.text_align;
        self.text_base = snapshot.text_base;
    }

    fn text_offsets(&self, text: &str, font: &Font, paint: &Paint) -> (f32, f32) {
        let (width, _bounds) = font.measure_str(text, Some(paint));
        let metrics = font.metrics().1;
        let dx = match self.text_align {
            TextAlign::Left => 0.0,
            TextAlign::Center => -width / 2.0,
            TextAlign::Right => -width,
        };
        let dy = match self.text_base {
            TextBase::Top => -metrics.ascent,
            TextBase::Middle => -(metrics.ascent + metrics.descent) / 2.0,
            TextBase::Alphabetic => 0.0,
            TextBase::Bottom => -metrics.descent,
        };
        (dx, dy)
    }
}

#[derive(Clone)]
struct DrawStateSnapshot {
    fill_color: Color,
    stroke_color: Color,
    stroke_width: f32,
    font_id: Option<String>,
    font_size: f32,
    text_align: TextAlign,
    text_base: TextBase,
}

impl Default for DrawStateSnapshot {
    fn default() -> Self {
        Self {
            fill_color: Color::BLACK,
            stroke_color: Color::BLACK,
            stroke_width: 1.0,
            font_id: None,
            font_size: DrawState::DEFAULT_FONT_SIZE,
            text_align: TextAlign::Left,
            text_base: TextBase::Alphabetic,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextBase {
    Top,
    Middle,
    Alphabetic,
    Bottom,
}
