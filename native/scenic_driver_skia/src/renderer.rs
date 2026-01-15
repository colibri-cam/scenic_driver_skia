use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use skia_safe::{
    AlphaType, ClipOp, Color, ColorType, Data, FilterMode, Font, FontMgr, FontStyle, Image,
    ImageInfo, Matrix, MipmapMode, Paint, PaintCap, PaintJoin, PaintStyle, PathBuilder,
    PathDirection, Point, RRect, Rect, SamplingOptions, Shader, Surface, TileMode, Typeface,
    Vector,
    canvas::SrcRectConstraint,
    gpu::{self, SurfaceOrigin, backend_render_targets, gl::FramebufferInfo},
    images,
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
    FillLinear {
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        start_color: Color,
        end_color: Color,
    },
    FillRadial {
        center_x: f32,
        center_y: f32,
        inner_radius: f32,
        outer_radius: f32,
        start_color: Color,
        end_color: Color,
    },
    StrokeLinear {
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        start_color: Color,
        end_color: Color,
    },
    StrokeRadial {
        center_x: f32,
        center_y: f32,
        inner_radius: f32,
        outer_radius: f32,
        start_color: Color,
        end_color: Color,
    },
    FillImage(String),
    FillStream(String),
    StrokeImage(String),
    StrokeStream(String),
    StrokeCap(PaintCap),
    StrokeJoin(PaintJoin),
    StrokeMiterLimit(f32),
    ClipPath(ClipOp),
    Scissor {
        width: f32,
        height: f32,
    },
    BeginPath,
    ClosePath,
    FillPath,
    StrokePath,
    MoveTo {
        x: f32,
        y: f32,
    },
    LineTo {
        x: f32,
        y: f32,
    },
    ArcTo {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        radius: f32,
    },
    BezierTo {
        cp1x: f32,
        cp1y: f32,
        cp2x: f32,
        cp2y: f32,
        x: f32,
        y: f32,
    },
    QuadraticTo {
        cpx: f32,
        cpy: f32,
        x: f32,
        y: f32,
    },
    PathTriangle {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    PathQuad {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
    },
    PathRect {
        width: f32,
        height: f32,
    },
    PathRRect {
        width: f32,
        height: f32,
        radius: f32,
    },
    PathSector {
        radius: f32,
        radians: f32,
    },
    PathCircle {
        radius: f32,
    },
    PathEllipse {
        radius0: f32,
        radius1: f32,
    },
    PathArc {
        cx: f32,
        cy: f32,
        radius: f32,
        start: f32,
        end: f32,
        dir: u32,
    },
    DrawLine {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        flag: u16,
    },
    DrawTriangle {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        flag: u16,
    },
    DrawQuad {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
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
    DrawSprites {
        image_id: String,
        cmds: Vec<SpriteCommand>,
    },
    DrawText(String),
    Font(String),
    FontSize(f32),
    TextAlign(TextAlign),
    TextBase(TextBase),
    DrawScript(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpriteCommand {
    pub sx: f32,
    pub sy: f32,
    pub sw: f32,
    pub sh: f32,
    pub dx: f32,
    pub dy: f32,
    pub dw: f32,
    pub dh: f32,
    pub alpha: f32,
}

#[derive(Clone, Debug)]
pub struct RenderState {
    pub clear_color: Color,
    pub scripts: HashMap<String, Vec<ScriptOp>>,
    pub root_id: Option<String>,
}

static IMAGE_CACHE: OnceLock<Mutex<HashMap<String, Image>>> = OnceLock::new();
static STREAM_CACHE: OnceLock<Mutex<HashMap<String, Image>>> = OnceLock::new();
static FONT_CACHE: OnceLock<Mutex<HashMap<String, Typeface>>> = OnceLock::new();

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
    scale_factor: f32,
}

impl Renderer {
    pub fn new(
        dimensions: (u32, u32),
        fb_info: FramebufferInfo,
        gr_context: skia_safe::gpu::DirectContext,
        num_samples: usize,
        stencil_size: usize,
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
            scale_factor: 1.0,
        }
    }

    pub fn from_surface(
        surface: Surface,
        gr_context: Option<skia_safe::gpu::DirectContext>,
    ) -> Self {
        Self {
            surface,
            gr_context,
            source: SurfaceSource::Raster,
            scale_factor: 1.0,
        }
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor.max(0.1);
    }

    pub fn surface_mut(&mut self) -> &mut Surface {
        &mut self.surface
    }

    pub fn redraw(&mut self, render_state: &RenderState) {
        let canvas = self.surface.canvas();
        canvas.clear(render_state.clear_color);

        canvas.save();
        if (self.scale_factor - 1.0).abs() > f32::EPSILON {
            canvas.scale((self.scale_factor, self.scale_factor));
        }

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

        canvas.restore();

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
            ScriptOp::FillColor(color) => {
                draw_state.fill_color = *color;
                draw_state.fill_shader = None;
            }
            ScriptOp::StrokeColor(color) => {
                draw_state.stroke_color = *color;
                draw_state.stroke_shader = None;
            }
            ScriptOp::StrokeWidth(width) => draw_state.stroke_width = *width,
            ScriptOp::FillLinear {
                start_x,
                start_y,
                end_x,
                end_y,
                start_color,
                end_color,
            } => {
                draw_state.fill_color = *start_color;
                let colors = [*start_color, *end_color];
                draw_state.fill_shader = Shader::linear_gradient(
                    (Point::new(*start_x, *start_y), Point::new(*end_x, *end_y)),
                    colors.as_slice(),
                    None,
                    TileMode::Clamp,
                    None,
                    None,
                );
            }
            ScriptOp::FillRadial {
                center_x,
                center_y,
                inner_radius,
                outer_radius,
                start_color,
                end_color,
            } => {
                draw_state.fill_color = *start_color;
                let colors = [*start_color, *end_color];
                draw_state.fill_shader = radial_shader(
                    *center_x,
                    *center_y,
                    *inner_radius,
                    *outer_radius,
                    colors.as_slice(),
                );
            }
            ScriptOp::StrokeLinear {
                start_x,
                start_y,
                end_x,
                end_y,
                start_color,
                end_color,
            } => {
                draw_state.stroke_color = *start_color;
                let colors = [*start_color, *end_color];
                draw_state.stroke_shader = Shader::linear_gradient(
                    (Point::new(*start_x, *start_y), Point::new(*end_x, *end_y)),
                    colors.as_slice(),
                    None,
                    TileMode::Clamp,
                    None,
                    None,
                );
            }
            ScriptOp::StrokeRadial {
                center_x,
                center_y,
                inner_radius,
                outer_radius,
                start_color,
                end_color,
            } => {
                draw_state.stroke_color = *start_color;
                let colors = [*start_color, *end_color];
                draw_state.stroke_shader = radial_shader(
                    *center_x,
                    *center_y,
                    *inner_radius,
                    *outer_radius,
                    colors.as_slice(),
                );
            }
            ScriptOp::FillImage(id) => {
                set_fill_image_shader(draw_state, load_static_shader(id.as_str()));
            }
            ScriptOp::FillStream(id) => {
                set_fill_image_shader(draw_state, load_stream_shader(id.as_str()));
            }
            ScriptOp::StrokeImage(id) => {
                set_stroke_image_shader(draw_state, load_static_shader(id.as_str()));
            }
            ScriptOp::StrokeStream(id) => {
                set_stroke_image_shader(draw_state, load_stream_shader(id.as_str()));
            }
            ScriptOp::StrokeCap(cap) => draw_state.stroke_cap = *cap,
            ScriptOp::StrokeJoin(join) => draw_state.stroke_join = *join,
            ScriptOp::StrokeMiterLimit(limit) => draw_state.stroke_miter_limit = *limit,
            ScriptOp::ClipPath(clip_op) => {
                if let Some(path) = draw_state.path.as_ref() {
                    let matrix = canvas.local_to_device();
                    let matrix_3x3 = matrix.to_m33();
                    let path = path.snapshot_and_transform(Some(&matrix_3x3));
                    canvas.reset_matrix();
                    canvas.clip_path(&path, *clip_op, true);
                    canvas.set_matrix(&matrix);
                }
            }
            ScriptOp::Scissor { width, height } => {
                let rect = Rect::from_xywh(0.0, 0.0, *width, *height);
                canvas.clip_rect(rect, ClipOp::Intersect, true);
            }
            ScriptOp::BeginPath => draw_state.path = Some(PathBuilder::new()),
            ScriptOp::ClosePath => {
                if let Some(path) = draw_state.path.as_mut() {
                    path.close();
                }
            }
            ScriptOp::FillPath => {
                if let Some(path) = draw_state.path.as_ref() {
                    let mut paint = Paint::default();
                    apply_fill_paint(&mut paint, draw_state);
                    let mut cloned = path.clone();
                    canvas.draw_path(&cloned.detach(), &paint);
                }
            }
            ScriptOp::StrokePath => {
                if let Some(mut path) = draw_state.path.take() {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
                    canvas.draw_path(&path.detach(), &paint);
                }
            }
            ScriptOp::MoveTo { x, y } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                path.move_to(Point::new(*x, *y));
            }
            ScriptOp::LineTo { x, y } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                path.line_to(Point::new(*x, *y));
            }
            ScriptOp::ArcTo {
                x1,
                y1,
                x2,
                y2,
                radius,
            } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                if !path.is_empty() {
                    path.arc_to_tangent(Point::new(*x1, *y1), Point::new(*x2, *y2), *radius);
                }
            }
            ScriptOp::BezierTo {
                cp1x,
                cp1y,
                cp2x,
                cp2y,
                x,
                y,
            } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                path.cubic_to(
                    Point::new(*cp1x, *cp1y),
                    Point::new(*cp2x, *cp2y),
                    Point::new(*x, *y),
                );
            }
            ScriptOp::QuadraticTo { cpx, cpy, x, y } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                path.quad_to(Point::new(*cpx, *cpy), Point::new(*x, *y));
            }
            ScriptOp::PathTriangle {
                x0,
                y0,
                x1,
                y1,
                x2,
                y2,
            } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                let points = [
                    Point::new(*x0, *y0),
                    Point::new(*x1, *y1),
                    Point::new(*x2, *y2),
                ];
                path.add_polygon(&points, true);
            }
            ScriptOp::PathQuad {
                x0,
                y0,
                x1,
                y1,
                x2,
                y2,
                x3,
                y3,
            } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                let points = [
                    Point::new(*x0, *y0),
                    Point::new(*x1, *y1),
                    Point::new(*x2, *y2),
                    Point::new(*x3, *y3),
                ];
                path.add_polygon(&points, true);
            }
            ScriptOp::PathRect { width, height } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                let rect = Rect::from_xywh(0.0, 0.0, *width, *height);
                path.add_rect(rect, PathDirection::CW, None);
            }
            ScriptOp::PathRRect {
                width,
                height,
                radius,
            } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                let rect = Rect::from_xywh(0.0, 0.0, *width, *height);
                let rrect = RRect::new_rect_xy(rect, *radius, *radius);
                path.add_rrect(rrect, PathDirection::CW, None);
            }
            ScriptOp::PathSector { radius, radians } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                let rect = Rect::from_xywh(-radius, -radius, radius * 2.0, radius * 2.0);
                let sweep = radians.to_degrees();
                path.move_to(Point::new(0.0, 0.0));
                path.line_to(Point::new(*radius, 0.0));
                path.arc_to(rect, 0.0, sweep, false);
                path.close();
            }
            ScriptOp::PathCircle { radius } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                path.add_circle(Point::new(0.0, 0.0), *radius, PathDirection::CW);
            }
            ScriptOp::PathEllipse { radius0, radius1 } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                let rect = Rect::from_xywh(-radius0, -radius1, radius0 * 2.0, radius1 * 2.0);
                path.add_oval(rect, PathDirection::CW, None);
            }
            ScriptOp::PathArc {
                cx,
                cy,
                radius,
                start,
                end,
                dir,
            } => {
                let path = draw_state.path.get_or_insert_with(PathBuilder::new);
                let rect = Rect::from_xywh(cx - radius, cy - radius, radius * 2.0, radius * 2.0);
                let mut sweep = (end - start).to_degrees();
                if *dir == 2 {
                    sweep = -sweep;
                }
                path.add_arc(rect, start.to_degrees(), sweep);
            }
            ScriptOp::DrawLine {
                x0,
                y0,
                x1,
                y1,
                flag,
            } => {
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
                    canvas.draw_line(Point::new(*x0, *y0), Point::new(*x1, *y1), &paint);
                }
            }
            ScriptOp::DrawTriangle {
                x0,
                y0,
                x1,
                y1,
                x2,
                y2,
                flag,
            } => {
                let mut builder = PathBuilder::new();
                builder
                    .move_to(Point::new(*x0, *y0))
                    .line_to(Point::new(*x1, *y1))
                    .line_to(Point::new(*x2, *y2))
                    .close();
                let path = builder.detach();
                if flag & 0x01 == 0x01 {
                    let mut paint = Paint::default();
                    apply_fill_paint(&mut paint, draw_state);
                    canvas.draw_path(&path, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
                    canvas.draw_path(&path, &paint);
                }
            }
            ScriptOp::DrawQuad {
                x0,
                y0,
                x1,
                y1,
                x2,
                y2,
                x3,
                y3,
                flag,
            } => {
                let mut builder = PathBuilder::new();
                builder
                    .move_to(Point::new(*x0, *y0))
                    .line_to(Point::new(*x1, *y1))
                    .line_to(Point::new(*x2, *y2))
                    .line_to(Point::new(*x3, *y3))
                    .close();
                let path = builder.detach();
                if flag & 0x01 == 0x01 {
                    let mut paint = Paint::default();
                    apply_fill_paint(&mut paint, draw_state);
                    canvas.draw_path(&path, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
                    canvas.draw_path(&path, &paint);
                }
            }
            ScriptOp::DrawCircle { radius, flag } => {
                if flag & 0x01 == 0x01 {
                    let mut paint = Paint::default();
                    apply_fill_paint(&mut paint, draw_state);
                    canvas.draw_circle(Point::new(0.0, 0.0), *radius, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
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
                    apply_fill_paint(&mut paint, draw_state);
                    canvas.draw_oval(rect, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
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
                    apply_fill_paint(&mut paint, draw_state);
                    canvas.draw_arc(rect, start, sweep, false, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
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
                    apply_fill_paint(&mut paint, draw_state);
                    canvas.draw_path(&path, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
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
                    apply_fill_paint(&mut paint, draw_state);
                    canvas.draw_rect(rect, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let rect = Rect::from_xywh(0.0, 0.0, *width, *height);
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
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
                    apply_fill_paint(&mut paint, draw_state);
                    canvas.draw_rrect(rrect, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
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
                    apply_fill_paint(&mut paint, draw_state);
                    canvas.draw_rrect(rrect, &paint);
                }
                if flag & 0x02 == 0x02 {
                    let mut paint = Paint::default();
                    apply_stroke_paint(&mut paint, draw_state);
                    canvas.draw_rrect(rrect, &paint);
                }
            }
            ScriptOp::DrawSprites { image_id, cmds } => {
                let Some(image) = cached_static_image(image_id.as_str()) else {
                    continue;
                };
                for cmd in cmds {
                    let src = Rect::from_xywh(cmd.sx, cmd.sy, cmd.sw, cmd.sh);
                    let dst = Rect::from_xywh(cmd.dx, cmd.dy, cmd.dw, cmd.dh);
                    let mut paint = Paint::default();
                    paint.set_alpha_f(cmd.alpha);
                    canvas.draw_image_rect_with_sampling_options(
                        &image,
                        Some((&src, SrcRectConstraint::Fast)),
                        dst,
                        SamplingOptions::new(FilterMode::Linear, MipmapMode::None),
                        &paint,
                    );
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
                    apply_fill_paint(&mut paint, draw_state);
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

fn apply_fill_paint(paint: &mut Paint, draw_state: &DrawState) {
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    if let Some(shader) = &draw_state.fill_shader {
        paint.set_shader(shader.clone());
        paint.set_color(Color::WHITE);
    } else {
        paint.set_color(draw_state.fill_color);
    }
}

fn apply_stroke_paint(paint: &mut Paint, draw_state: &DrawState) {
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(draw_state.stroke_width);
    paint.set_stroke_cap(draw_state.stroke_cap);
    paint.set_stroke_join(draw_state.stroke_join);
    paint.set_stroke_miter(draw_state.stroke_miter_limit);
    if let Some(shader) = &draw_state.stroke_shader {
        paint.set_shader(shader.clone());
        paint.set_color(Color::WHITE);
    } else {
        paint.set_color(draw_state.stroke_color);
    }
}

fn set_fill_image_shader(draw_state: &mut DrawState, shader: Option<Shader>) {
    if let Some(shader) = shader {
        draw_state.fill_shader = Some(shader);
        draw_state.fill_color = Color::WHITE;
    } else {
        draw_state.fill_shader = None;
        draw_state.fill_color = Color::TRANSPARENT;
    }
}

fn set_stroke_image_shader(draw_state: &mut DrawState, shader: Option<Shader>) {
    if let Some(shader) = shader {
        draw_state.stroke_shader = Some(shader);
        draw_state.stroke_color = Color::WHITE;
    } else {
        draw_state.stroke_shader = None;
        draw_state.stroke_color = Color::TRANSPARENT;
    }
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
    let cache = FONT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(cache) = cache.lock()
        && let Some(typeface) = cache.get(font_id)
    {
        return Some(typeface.clone());
    }
    None
}

pub fn insert_font(id: &str, data: &[u8]) -> Result<(), String> {
    let fm = FontMgr::new();
    let typeface = fm
        .new_from_data(data, 0)
        .ok_or_else(|| "invalid font data".to_string())?;
    let cache = FONT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .map_err(|_| "font cache lock poisoned".to_string())?;
    cache.insert(id.to_string(), typeface);
    Ok(())
}

fn load_static_shader(id: &str) -> Option<Shader> {
    cached_static_image(id).and_then(|image| image_to_shader(&image))
}

fn load_stream_shader(id: &str) -> Option<Shader> {
    cached_stream_image(id).and_then(|image| image_to_shader(&image))
}

fn image_to_shader(image: &Image) -> Option<Shader> {
    image.to_shader(
        Some((TileMode::Repeat, TileMode::Repeat)),
        SamplingOptions::new(FilterMode::Linear, MipmapMode::None),
        None,
    )
}

fn radial_shader(
    center_x: f32,
    center_y: f32,
    inner_radius: f32,
    outer_radius: f32,
    colors: &[Color],
) -> Option<Shader> {
    if inner_radius <= 0.0 {
        Shader::radial_gradient(
            Point::new(center_x, center_y),
            outer_radius,
            colors,
            None,
            TileMode::Clamp,
            None,
            None,
        )
    } else {
        Shader::two_point_conical_gradient(
            Point::new(center_x, center_y),
            inner_radius,
            Point::new(center_x, center_y),
            outer_radius,
            colors,
            None,
            TileMode::Clamp,
            None,
            None,
        )
    }
}

fn cached_static_image(id: &str) -> Option<Image> {
    let cache = IMAGE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(cache) = cache.lock()
        && let Some(image) = cache.get(id)
    {
        return Some(image.clone());
    }
    None
}

fn cached_stream_image(id: &str) -> Option<Image> {
    let cache = STREAM_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(cache) = cache.lock()
        && let Some(image) = cache.get(id)
    {
        return Some(image.clone());
    }

    None
}

pub fn insert_static_image(id: &str, image: Image) {
    let cache = IMAGE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(mut cache) = cache.lock() {
        cache.insert(id.to_string(), image);
    }
}

pub fn insert_stream_image(id: &str, image: Image) {
    let cache = STREAM_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(mut cache) = cache.lock() {
        cache.insert(id.to_string(), image);
    }
}

pub fn remove_stream_image(id: &str) {
    let cache = STREAM_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(mut cache) = cache.lock() {
        cache.remove(id);
    }
}

pub fn decode_texture_image(
    format: &str,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<Image, String> {
    if format == "file" {
        return Image::from_encoded(Data::new_copy(data))
            .ok_or_else(|| "failed to decode image data".to_string());
    }

    let pixel_count = width
        .checked_mul(height)
        .ok_or_else(|| "texture dimensions overflow".to_string())?;
    let pixel_count = pixel_count as usize;

    let rgba = match format {
        "g" => {
            if data.len() != pixel_count {
                return Err("gray bitmap size mismatch".to_string());
            }
            let mut rgba = Vec::with_capacity(pixel_count * 4);
            for &g in data {
                rgba.extend_from_slice(&[g, g, g, 0xFF]);
            }
            rgba
        }
        "ga" => {
            if data.len() != pixel_count * 2 {
                return Err("ga bitmap size mismatch".to_string());
            }
            let mut rgba = Vec::with_capacity(pixel_count * 4);
            for chunk in data.chunks_exact(2) {
                let g = chunk[0];
                let a = chunk[1];
                rgba.extend_from_slice(&[g, g, g, a]);
            }
            rgba
        }
        "rgb" => {
            if data.len() != pixel_count * 3 {
                return Err("rgb bitmap size mismatch".to_string());
            }
            let mut rgba = Vec::with_capacity(pixel_count * 4);
            for chunk in data.chunks_exact(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 0xFF]);
            }
            rgba
        }
        "rgba" => {
            if data.len() != pixel_count * 4 {
                return Err("rgba bitmap size mismatch".to_string());
            }
            data.to_vec()
        }
        _ => return Err(format!("unsupported texture format: {format}")),
    };

    let info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );
    let row_bytes = (width as usize) * 4;
    let data = Data::new_copy(&rgba);
    images::raster_from_data(&info, data, row_bytes)
        .ok_or_else(|| "failed to build raster image".to_string())
}

#[derive(Clone)]
struct DrawState {
    fill_color: Color,
    fill_shader: Option<Shader>,
    stroke_color: Color,
    stroke_shader: Option<Shader>,
    stroke_width: f32,
    stroke_cap: PaintCap,
    stroke_join: PaintJoin,
    stroke_miter_limit: f32,
    path: Option<PathBuilder>,
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
            fill_shader: None,
            stroke_color: Color::BLACK,
            stroke_shader: None,
            stroke_width: 1.0,
            stroke_cap: PaintCap::Butt,
            stroke_join: PaintJoin::Miter,
            stroke_miter_limit: 4.0,
            path: None,
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
            fill_shader: self.fill_shader.clone(),
            stroke_color: self.stroke_color,
            stroke_shader: self.stroke_shader.clone(),
            stroke_width: self.stroke_width,
            stroke_cap: self.stroke_cap,
            stroke_join: self.stroke_join,
            stroke_miter_limit: self.stroke_miter_limit,
            path: self.path.clone(),
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
        self.fill_shader = snapshot.fill_shader;
        self.stroke_color = snapshot.stroke_color;
        self.stroke_shader = snapshot.stroke_shader;
        self.stroke_width = snapshot.stroke_width;
        self.stroke_cap = snapshot.stroke_cap;
        self.stroke_join = snapshot.stroke_join;
        self.stroke_miter_limit = snapshot.stroke_miter_limit;
        self.path = snapshot.path;
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
    fill_shader: Option<Shader>,
    stroke_color: Color,
    stroke_shader: Option<Shader>,
    stroke_width: f32,
    stroke_cap: PaintCap,
    stroke_join: PaintJoin,
    stroke_miter_limit: f32,
    path: Option<PathBuilder>,
    font_id: Option<String>,
    font_size: f32,
    text_align: TextAlign,
    text_base: TextBase,
}

impl Default for DrawStateSnapshot {
    fn default() -> Self {
        Self {
            fill_color: Color::BLACK,
            fill_shader: None,
            stroke_color: Color::BLACK,
            stroke_shader: None,
            stroke_width: 1.0,
            stroke_cap: PaintCap::Butt,
            stroke_join: PaintJoin::Miter,
            stroke_miter_limit: 4.0,
            path: None,
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
