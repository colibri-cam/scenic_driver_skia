mod backend;
mod cursor;
mod drm_backend;
mod drm_input;
mod input;
mod input_translate;
mod raster_backend;
mod renderer;

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc,
};
use std::thread;
use std::time::Duration;

use backend::UserEvent;
use cursor::CursorState;
use input::{InputEvent, InputQueue, notify_input_ready};
use renderer::{RenderState, ScriptOp};
use rustler::{Binary, Env, OwnedBinary, ResourceArc, Term};
use skia_safe::ClipOp;

enum StopSignal {
    Wayland(winit::event_loop::EventLoopProxy<UserEvent>),
    Drm(Arc<AtomicBool>),
    Raster(Arc<AtomicBool>),
}

struct DriverHandle {
    stop: StopSignal,
    text: Arc<Mutex<String>>,
    render_state: Arc<Mutex<RenderState>>,
    input_events: Arc<Mutex<InputQueue>>,
    input_mask: Arc<AtomicU32>,
    raster_frame: Option<Arc<Mutex<Option<RasterFrame>>>>,
    dirty: Option<Arc<AtomicBool>>,
    running: Arc<AtomicBool>,
    cursor_state: Option<Arc<Mutex<CursorState>>>,
    thread: Option<thread::JoinHandle<()>>,
}

struct RendererResource {
    handle: Mutex<DriverHandle>,
}

impl rustler::Resource for RendererResource {}

pub(crate) struct RasterFrame {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

const ROOT_ID: &str = "_root_";

#[rustler::nif(schedule = "DirtyIo")]
pub fn start(
    backend: Option<String>,
    viewport_size: Option<(u32, u32)>,
    window_title: String,
    window_resizeable: bool,
    drm_card: Option<String>,
    drm_hw_cursor: bool,
    drm_input_log: bool,
) -> Result<ResourceArc<RendererResource>, String> {
    let backend = backend
        .map(|b| b.to_lowercase())
        .unwrap_or_else(|| String::from("wayland"));

    let thread_name = format!("scenic-driver-{backend}");
    let text = Arc::new(Mutex::new(String::from("Hello, Wayland")));
    let render_state = Arc::new(Mutex::new(RenderState::default()));
    let input_events = Arc::new(Mutex::new(InputQueue::new()));
    let input_mask = Arc::new(AtomicU32::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let handle = if backend == "drm" {
        let stop = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(false));
        let text_for_thread = Arc::clone(&text);
        let state_for_thread = Arc::clone(&render_state);
        let dirty_for_thread = Arc::clone(&dirty);
        let stop_for_thread = Arc::clone(&stop);
        let input_for_thread = Arc::clone(&input_mask);
        let input_events_for_thread = Arc::clone(&input_events);
        let requested_size = viewport_size;
        let cursor_state = Arc::new(Mutex::new(CursorState::new()));
        let cursor_for_thread = Arc::clone(&cursor_state);
        let drm_card = drm_card.clone();
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                drm_backend::run(
                    stop_for_thread,
                    text_for_thread,
                    dirty_for_thread,
                    state_for_thread,
                    input_for_thread,
                    input_events_for_thread,
                    drm_backend::DrmRunConfig {
                        requested_size,
                        cursor_state: cursor_for_thread,
                        card_path: drm_card,
                        hw_cursor: drm_hw_cursor,
                        input_log: drm_input_log,
                    },
                )
            })
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        DriverHandle {
            stop: StopSignal::Drm(stop),
            text,
            render_state,
            input_events,
            input_mask,
            raster_frame: None,
            dirty: Some(dirty),
            running,
            cursor_state: Some(cursor_state),
            thread: Some(thread),
        }
    } else if backend == "raster" {
        let stop = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(false));
        let state_for_thread = Arc::clone(&render_state);
        let dirty_for_thread = Arc::clone(&dirty);
        let stop_for_thread = Arc::clone(&stop);
        let text_for_thread = Arc::clone(&text);
        let raster_frame = Arc::new(Mutex::new(None));
        let frame_for_thread = Arc::clone(&raster_frame);
        let input_for_thread = Arc::clone(&input_mask);
        let requested_size = viewport_size;
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                raster_backend::run(
                    stop_for_thread,
                    dirty_for_thread,
                    state_for_thread,
                    frame_for_thread,
                    text_for_thread,
                    input_for_thread,
                    requested_size,
                )
            })
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        DriverHandle {
            stop: StopSignal::Raster(stop),
            text,
            render_state,
            input_events,
            input_mask,
            raster_frame: Some(raster_frame),
            dirty: Some(dirty),
            running,
            cursor_state: None,
            thread: Some(thread),
        }
    } else {
        let (proxy_tx, proxy_rx) = mpsc::channel();
        let initial_text = text
            .lock()
            .map_err(|_| "driver state lock poisoned".to_string())?
            .clone();
        let running_for_thread = Arc::clone(&running);
        let state_for_thread = Arc::clone(&render_state);
        let input_for_thread = Arc::clone(&input_mask);
        let input_events_for_thread = Arc::clone(&input_events);
        let requested_size = viewport_size;
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                backend::run(
                    proxy_tx,
                    initial_text,
                    running_for_thread,
                    state_for_thread,
                    input_for_thread,
                    input_events_for_thread,
                    backend::WaylandWindowConfig {
                        requested_size,
                        window_title,
                        window_resizeable,
                    },
                )
            })
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        let proxy = proxy_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "renderer did not initialize in time".to_string())?;
        DriverHandle {
            stop: StopSignal::Wayland(proxy),
            text,
            render_state,
            input_events,
            input_mask,
            raster_frame: None,
            dirty: None,
            running,
            cursor_state: None,
            thread: Some(thread),
        }
    };

    Ok(ResourceArc::new(RendererResource {
        handle: Mutex::new(handle),
    }))
}

fn with_handle<T>(
    renderer: &RendererResource,
    f: impl FnOnce(&mut DriverHandle) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = renderer
        .handle
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    f(&mut guard)
}

fn signal_redraw(handle: &mut DriverHandle) -> Result<(), String> {
    match &handle.stop {
        StopSignal::Wayland(proxy) => proxy
            .send_event(UserEvent::Redraw)
            .map_err(|err| format!("failed to signal renderer: {err}")),
        StopSignal::Drm(_) | StopSignal::Raster(_) => {
            if let Some(dirty) = &handle.dirty {
                dirty.store(true, Ordering::Relaxed);
            }
            Ok(())
        }
    }
}

fn update_render_state<F>(renderer: &RendererResource, update: F) -> Result<(), String>
where
    F: FnOnce(&mut RenderState) -> Result<(), String>,
{
    with_handle(renderer, |handle| {
        let mut render_state = handle
            .render_state
            .lock()
            .map_err(|_| "render state lock poisoned".to_string())?;
        update(&mut render_state)?;
        drop(render_state);
        signal_redraw(handle)
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn stop(renderer: ResourceArc<RendererResource>) -> Result<(), String> {
    with_handle(&renderer, |handle| {
        if !handle.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        let signal_result = match &handle.stop {
            StopSignal::Wayland(proxy) => proxy
                .send_event(UserEvent::Stop)
                .map_err(|err| format!("failed to signal renderer: {err}")),
            StopSignal::Drm(stop) => {
                stop.store(true, Ordering::Relaxed);
                Ok(())
            }
            StopSignal::Raster(stop) => {
                stop.store(true, Ordering::Relaxed);
                Ok(())
            }
        };
        handle.running.store(false, Ordering::Relaxed);

        let join_result = match handle.thread.take() {
            Some(thread) => thread
                .join()
                .map_err(|_| "renderer thread panicked".to_string()),
            None => Ok(()),
        };

        signal_result.and(join_result)
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_text(renderer: ResourceArc<RendererResource>, text: String) -> Result<(), String> {
    with_handle(&renderer, |handle| {
        {
            let mut stored = handle
                .text
                .lock()
                .map_err(|_| "text state lock poisoned".to_string())?;
            *stored = text.clone();
        }

        match &handle.stop {
            StopSignal::Wayland(proxy) => proxy
                .send_event(UserEvent::SetText(text))
                .map_err(|err| format!("failed to signal renderer: {err}")),
            StopSignal::Drm(_) | StopSignal::Raster(_) => {
                if let Some(dirty) = &handle.dirty {
                    dirty.store(true, Ordering::Relaxed);
                }
                Ok(())
            }
        }
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn reset_scene(renderer: ResourceArc<RendererResource>) -> Result<(), String> {
    update_render_state(&renderer, |state| {
        state.scripts = HashMap::new();
        state.root_id = None;
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_clear_color(
    renderer: ResourceArc<RendererResource>,
    color: (u8, u8, u8, u8),
) -> Result<(), String> {
    update_render_state(&renderer, |state| {
        state.clear_color = skia_safe::Color::from_argb(color.3, color.0, color.1, color.2);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn submit_script(
    renderer: ResourceArc<RendererResource>,
    script: rustler::Binary,
) -> Result<(), String> {
    update_render_state(&renderer, |state| {
        let ops = parse_script(script.as_slice())?;
        set_script(state, ROOT_ID.to_string(), ops);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn submit_script_with_id(
    renderer: ResourceArc<RendererResource>,
    id: String,
    script: rustler::Binary,
) -> Result<(), String> {
    update_render_state(&renderer, |state| {
        let ops = parse_script(script.as_slice())?;
        set_script(state, id.clone(), ops);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn submit_scripts(
    renderer: ResourceArc<RendererResource>,
    scripts: Vec<(String, rustler::Binary)>,
) -> Result<(), String> {
    update_render_state(&renderer, |state| {
        let mut staged: Vec<(String, Vec<ScriptOp>)> = Vec::with_capacity(scripts.len());
        for (id, script) in scripts.iter() {
            let ops = parse_script(script.as_slice())?;
            staged.push((id.clone(), ops));
        }
        for (id, ops) in staged {
            set_script(state, id, ops);
        }
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn put_static_image(
    renderer: ResourceArc<RendererResource>,
    id: String,
    data: rustler::Binary,
) -> Result<(), String> {
    let image = renderer::decode_texture_image("file", 0, 0, data.as_slice())?;
    renderer::insert_static_image(&id, image);
    with_handle(&renderer, signal_redraw)
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn put_stream_texture(
    renderer: ResourceArc<RendererResource>,
    id: String,
    format: String,
    width: u32,
    height: u32,
    data: rustler::Binary,
) -> Result<(), String> {
    let image = renderer::decode_texture_image(&format, width, height, data.as_slice())?;
    renderer::insert_stream_image(&id, image);
    with_handle(&renderer, signal_redraw)
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn del_stream_texture(
    renderer: ResourceArc<RendererResource>,
    id: String,
) -> Result<(), String> {
    renderer::remove_stream_image(&id);
    with_handle(&renderer, signal_redraw)
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn del_script(renderer: ResourceArc<RendererResource>, id: String) -> Result<(), String> {
    update_render_state(&renderer, |state| {
        state.scripts.remove(&id);
        if state.root_id.as_deref() == Some(id.as_str()) {
            state.root_id = None;
        }
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn script_count(renderer: ResourceArc<RendererResource>) -> Result<u64, String> {
    with_handle(&renderer, |handle| {
        let render_state = handle
            .render_state
            .lock()
            .map_err(|_| "render state lock poisoned".to_string())?;
        Ok(render_state.scripts.len() as u64)
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn get_raster_frame<'a>(
    env: Env<'a>,
    renderer: ResourceArc<RendererResource>,
) -> Result<(u32, u32, Binary<'a>), String> {
    with_handle(&renderer, |handle| {
        let frame_slot = handle
            .raster_frame
            .as_ref()
            .ok_or_else(|| "raster backend not active".to_string())?;
        let frame_guard = frame_slot
            .lock()
            .map_err(|_| "raster frame lock poisoned".to_string())?;
        let frame = frame_guard
            .as_ref()
            .ok_or_else(|| "raster frame not available".to_string())?;
        let mut binary = OwnedBinary::new(frame.data.len())
            .ok_or_else(|| "failed to allocate raster frame binary".to_string())?;
        binary.as_mut_slice().copy_from_slice(&frame.data);
        Ok((frame.width, frame.height, Binary::from_owned(binary, env)))
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_input_mask(renderer: ResourceArc<RendererResource>, mask: u32) -> Result<(), String> {
    with_handle(&renderer, |handle| {
        handle.input_mask.store(mask, Ordering::Relaxed);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn show_cursor(renderer: ResourceArc<RendererResource>) -> Result<(), String> {
    set_cursor_visible(&renderer, true)
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn hide_cursor(renderer: ResourceArc<RendererResource>) -> Result<(), String> {
    set_cursor_visible(&renderer, false)
}

fn set_cursor_visible(renderer: &RendererResource, visible: bool) -> Result<(), String> {
    with_handle(renderer, |handle| {
        if let Some(cursor_state) = &handle.cursor_state
            && let Ok(mut cursor) = cursor_state.lock()
        {
            cursor.visible = visible;
        }

        if let Some(dirty) = &handle.dirty {
            dirty.store(true, Ordering::Relaxed);
        }

        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_input_target(
    renderer: ResourceArc<RendererResource>,
    pid: Option<rustler::LocalPid>,
) -> Result<(), String> {
    with_handle(&renderer, |handle| {
        let mut queue = handle
            .input_events
            .lock()
            .map_err(|_| "input queue lock poisoned".to_string())?;
        let notify = queue.set_target(pid);
        drop(queue);

        if let Some(pid) = notify {
            notify_input_ready(pid);
        }

        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn drain_input_events(
    renderer: ResourceArc<RendererResource>,
) -> Result<Vec<InputEvent>, String> {
    drain_input_events_inner(&renderer)
}

fn drain_input_events_inner(renderer: &RendererResource) -> Result<Vec<InputEvent>, String> {
    with_handle(renderer, |handle| {
        let mut queue = handle
            .input_events
            .lock()
            .map_err(|_| "input queue lock poisoned".to_string())?;
        Ok(queue.drain())
    })
}

fn set_script(state: &mut RenderState, id: String, ops: Vec<ScriptOp>) {
    state.scripts.insert(id.clone(), ops);
    if id == ROOT_ID {
        state.root_id = Some(id);
    }
}

fn parse_script(script: &[u8]) -> Result<Vec<ScriptOp>, String> {
    let mut rest = script;
    let mut ops = Vec::new();
    while rest.len() >= 2 {
        let (op, remaining) = rest.split_at(2);
        let opcode = u16::from_be_bytes([op[0], op[1]]);
        rest = remaining;
        match opcode {
            0x44 => {
                if rest.len() < 10 {
                    return Err("scissor opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (w_bytes, tail) = tail.split_at(4);
                let (h_bytes, tail) = tail.split_at(4);
                let width = f32::from_bits(u32::from_be_bytes([
                    w_bytes[0], w_bytes[1], w_bytes[2], w_bytes[3],
                ]));
                let height = f32::from_bits(u32::from_be_bytes([
                    h_bytes[0], h_bytes[1], h_bytes[2], h_bytes[3],
                ]));
                ops.push(ScriptOp::Scissor { width, height });
                rest = tail;
            }
            0x45 => {
                if rest.len() < 2 {
                    return Err("clip_path opcode truncated".to_string());
                }
                let (mode_bytes, tail) = rest.split_at(2);
                let mode = u16::from_be_bytes([mode_bytes[0], mode_bytes[1]]);
                let clip_op = match mode {
                    0x00 => ClipOp::Intersect,
                    0x01 => ClipOp::Difference,
                    _ => return Err("clip_path opcode invalid".to_string()),
                };
                ops.push(ScriptOp::ClipPath(clip_op));
                rest = tail;
            }
            0x20 => {
                if rest.len() < 2 {
                    return Err("begin_path opcode truncated".to_string());
                }
                ops.push(ScriptOp::BeginPath);
                rest = &rest[2..];
            }
            0x21 => {
                if rest.len() < 2 {
                    return Err("close_path opcode truncated".to_string());
                }
                ops.push(ScriptOp::ClosePath);
                rest = &rest[2..];
            }
            0x22 => {
                if rest.len() < 2 {
                    return Err("fill_path opcode truncated".to_string());
                }
                ops.push(ScriptOp::FillPath);
                rest = &rest[2..];
            }
            0x23 => {
                if rest.len() < 2 {
                    return Err("stroke_path opcode truncated".to_string());
                }
                ops.push(ScriptOp::StrokePath);
                rest = &rest[2..];
            }
            0x26 => {
                if rest.len() < 10 {
                    return Err("move_to opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (x_bytes, tail) = tail.split_at(4);
                let (y_bytes, tail) = tail.split_at(4);
                let x = f32::from_bits(u32::from_be_bytes([
                    x_bytes[0], x_bytes[1], x_bytes[2], x_bytes[3],
                ]));
                let y = f32::from_bits(u32::from_be_bytes([
                    y_bytes[0], y_bytes[1], y_bytes[2], y_bytes[3],
                ]));
                ops.push(ScriptOp::MoveTo { x, y });
                rest = tail;
            }
            0x27 => {
                if rest.len() < 10 {
                    return Err("line_to opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (x_bytes, tail) = tail.split_at(4);
                let (y_bytes, tail) = tail.split_at(4);
                let x = f32::from_bits(u32::from_be_bytes([
                    x_bytes[0], x_bytes[1], x_bytes[2], x_bytes[3],
                ]));
                let y = f32::from_bits(u32::from_be_bytes([
                    y_bytes[0], y_bytes[1], y_bytes[2], y_bytes[3],
                ]));
                ops.push(ScriptOp::LineTo { x, y });
                rest = tail;
            }
            0x28 => {
                if rest.len() < 22 {
                    return Err("arc_to opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (x1_bytes, tail) = tail.split_at(4);
                let (y1_bytes, tail) = tail.split_at(4);
                let (x2_bytes, tail) = tail.split_at(4);
                let (y2_bytes, tail) = tail.split_at(4);
                let (r_bytes, tail) = tail.split_at(4);
                let x1 = f32::from_bits(u32::from_be_bytes([
                    x1_bytes[0],
                    x1_bytes[1],
                    x1_bytes[2],
                    x1_bytes[3],
                ]));
                let y1 = f32::from_bits(u32::from_be_bytes([
                    y1_bytes[0],
                    y1_bytes[1],
                    y1_bytes[2],
                    y1_bytes[3],
                ]));
                let x2 = f32::from_bits(u32::from_be_bytes([
                    x2_bytes[0],
                    x2_bytes[1],
                    x2_bytes[2],
                    x2_bytes[3],
                ]));
                let y2 = f32::from_bits(u32::from_be_bytes([
                    y2_bytes[0],
                    y2_bytes[1],
                    y2_bytes[2],
                    y2_bytes[3],
                ]));
                let radius = f32::from_bits(u32::from_be_bytes([
                    r_bytes[0], r_bytes[1], r_bytes[2], r_bytes[3],
                ]));
                ops.push(ScriptOp::ArcTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    radius,
                });
                rest = tail;
            }
            0x29 => {
                if rest.len() < 26 {
                    return Err("bezier_to opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (cp1x_bytes, tail) = tail.split_at(4);
                let (cp1y_bytes, tail) = tail.split_at(4);
                let (cp2x_bytes, tail) = tail.split_at(4);
                let (cp2y_bytes, tail) = tail.split_at(4);
                let (x_bytes, tail) = tail.split_at(4);
                let (y_bytes, tail) = tail.split_at(4);
                let cp1x = f32::from_bits(u32::from_be_bytes([
                    cp1x_bytes[0],
                    cp1x_bytes[1],
                    cp1x_bytes[2],
                    cp1x_bytes[3],
                ]));
                let cp1y = f32::from_bits(u32::from_be_bytes([
                    cp1y_bytes[0],
                    cp1y_bytes[1],
                    cp1y_bytes[2],
                    cp1y_bytes[3],
                ]));
                let cp2x = f32::from_bits(u32::from_be_bytes([
                    cp2x_bytes[0],
                    cp2x_bytes[1],
                    cp2x_bytes[2],
                    cp2x_bytes[3],
                ]));
                let cp2y = f32::from_bits(u32::from_be_bytes([
                    cp2y_bytes[0],
                    cp2y_bytes[1],
                    cp2y_bytes[2],
                    cp2y_bytes[3],
                ]));
                let x = f32::from_bits(u32::from_be_bytes([
                    x_bytes[0], x_bytes[1], x_bytes[2], x_bytes[3],
                ]));
                let y = f32::from_bits(u32::from_be_bytes([
                    y_bytes[0], y_bytes[1], y_bytes[2], y_bytes[3],
                ]));
                ops.push(ScriptOp::BezierTo {
                    cp1x,
                    cp1y,
                    cp2x,
                    cp2y,
                    x,
                    y,
                });
                rest = tail;
            }
            0x2A => {
                if rest.len() < 18 {
                    return Err("quadratic_to opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (cpx_bytes, tail) = tail.split_at(4);
                let (cpy_bytes, tail) = tail.split_at(4);
                let (x_bytes, tail) = tail.split_at(4);
                let (y_bytes, tail) = tail.split_at(4);
                let cpx = f32::from_bits(u32::from_be_bytes([
                    cpx_bytes[0],
                    cpx_bytes[1],
                    cpx_bytes[2],
                    cpx_bytes[3],
                ]));
                let cpy = f32::from_bits(u32::from_be_bytes([
                    cpy_bytes[0],
                    cpy_bytes[1],
                    cpy_bytes[2],
                    cpy_bytes[3],
                ]));
                let x = f32::from_bits(u32::from_be_bytes([
                    x_bytes[0], x_bytes[1], x_bytes[2], x_bytes[3],
                ]));
                let y = f32::from_bits(u32::from_be_bytes([
                    y_bytes[0], y_bytes[1], y_bytes[2], y_bytes[3],
                ]));
                ops.push(ScriptOp::QuadraticTo { cpx, cpy, x, y });
                rest = tail;
            }
            0x2B => {
                if rest.len() < 26 {
                    return Err("triangle opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (x0_bytes, tail) = tail.split_at(4);
                let (y0_bytes, tail) = tail.split_at(4);
                let (x1_bytes, tail) = tail.split_at(4);
                let (y1_bytes, tail) = tail.split_at(4);
                let (x2_bytes, tail) = tail.split_at(4);
                let (y2_bytes, tail) = tail.split_at(4);
                let x0 = f32::from_bits(u32::from_be_bytes([
                    x0_bytes[0],
                    x0_bytes[1],
                    x0_bytes[2],
                    x0_bytes[3],
                ]));
                let y0 = f32::from_bits(u32::from_be_bytes([
                    y0_bytes[0],
                    y0_bytes[1],
                    y0_bytes[2],
                    y0_bytes[3],
                ]));
                let x1 = f32::from_bits(u32::from_be_bytes([
                    x1_bytes[0],
                    x1_bytes[1],
                    x1_bytes[2],
                    x1_bytes[3],
                ]));
                let y1 = f32::from_bits(u32::from_be_bytes([
                    y1_bytes[0],
                    y1_bytes[1],
                    y1_bytes[2],
                    y1_bytes[3],
                ]));
                let x2 = f32::from_bits(u32::from_be_bytes([
                    x2_bytes[0],
                    x2_bytes[1],
                    x2_bytes[2],
                    x2_bytes[3],
                ]));
                let y2 = f32::from_bits(u32::from_be_bytes([
                    y2_bytes[0],
                    y2_bytes[1],
                    y2_bytes[2],
                    y2_bytes[3],
                ]));
                ops.push(ScriptOp::PathTriangle {
                    x0,
                    y0,
                    x1,
                    y1,
                    x2,
                    y2,
                });
                rest = tail;
            }
            0x2C => {
                if rest.len() < 34 {
                    return Err("quad opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (x0_bytes, tail) = tail.split_at(4);
                let (y0_bytes, tail) = tail.split_at(4);
                let (x1_bytes, tail) = tail.split_at(4);
                let (y1_bytes, tail) = tail.split_at(4);
                let (x2_bytes, tail) = tail.split_at(4);
                let (y2_bytes, tail) = tail.split_at(4);
                let (x3_bytes, tail) = tail.split_at(4);
                let (y3_bytes, tail) = tail.split_at(4);
                let x0 = f32::from_bits(u32::from_be_bytes([
                    x0_bytes[0],
                    x0_bytes[1],
                    x0_bytes[2],
                    x0_bytes[3],
                ]));
                let y0 = f32::from_bits(u32::from_be_bytes([
                    y0_bytes[0],
                    y0_bytes[1],
                    y0_bytes[2],
                    y0_bytes[3],
                ]));
                let x1 = f32::from_bits(u32::from_be_bytes([
                    x1_bytes[0],
                    x1_bytes[1],
                    x1_bytes[2],
                    x1_bytes[3],
                ]));
                let y1 = f32::from_bits(u32::from_be_bytes([
                    y1_bytes[0],
                    y1_bytes[1],
                    y1_bytes[2],
                    y1_bytes[3],
                ]));
                let x2 = f32::from_bits(u32::from_be_bytes([
                    x2_bytes[0],
                    x2_bytes[1],
                    x2_bytes[2],
                    x2_bytes[3],
                ]));
                let y2 = f32::from_bits(u32::from_be_bytes([
                    y2_bytes[0],
                    y2_bytes[1],
                    y2_bytes[2],
                    y2_bytes[3],
                ]));
                let x3 = f32::from_bits(u32::from_be_bytes([
                    x3_bytes[0],
                    x3_bytes[1],
                    x3_bytes[2],
                    x3_bytes[3],
                ]));
                let y3 = f32::from_bits(u32::from_be_bytes([
                    y3_bytes[0],
                    y3_bytes[1],
                    y3_bytes[2],
                    y3_bytes[3],
                ]));
                ops.push(ScriptOp::PathQuad {
                    x0,
                    y0,
                    x1,
                    y1,
                    x2,
                    y2,
                    x3,
                    y3,
                });
                rest = tail;
            }
            0x2D => {
                if rest.len() < 10 {
                    return Err("rect opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (w_bytes, tail) = tail.split_at(4);
                let (h_bytes, tail) = tail.split_at(4);
                let width = f32::from_bits(u32::from_be_bytes([
                    w_bytes[0], w_bytes[1], w_bytes[2], w_bytes[3],
                ]));
                let height = f32::from_bits(u32::from_be_bytes([
                    h_bytes[0], h_bytes[1], h_bytes[2], h_bytes[3],
                ]));
                ops.push(ScriptOp::PathRect { width, height });
                rest = tail;
            }
            0x2E => {
                if rest.len() < 14 {
                    return Err("rrect opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (w_bytes, tail) = tail.split_at(4);
                let (h_bytes, tail) = tail.split_at(4);
                let (r_bytes, tail) = tail.split_at(4);
                let width = f32::from_bits(u32::from_be_bytes([
                    w_bytes[0], w_bytes[1], w_bytes[2], w_bytes[3],
                ]));
                let height = f32::from_bits(u32::from_be_bytes([
                    h_bytes[0], h_bytes[1], h_bytes[2], h_bytes[3],
                ]));
                let radius = f32::from_bits(u32::from_be_bytes([
                    r_bytes[0], r_bytes[1], r_bytes[2], r_bytes[3],
                ]));
                ops.push(ScriptOp::PathRRect {
                    width,
                    height,
                    radius,
                });
                rest = tail;
            }
            0x2F => {
                if rest.len() < 10 {
                    return Err("sector opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (r_bytes, tail) = tail.split_at(4);
                let (rad_bytes, tail) = tail.split_at(4);
                let radius = f32::from_bits(u32::from_be_bytes([
                    r_bytes[0], r_bytes[1], r_bytes[2], r_bytes[3],
                ]));
                let radians = f32::from_bits(u32::from_be_bytes([
                    rad_bytes[0],
                    rad_bytes[1],
                    rad_bytes[2],
                    rad_bytes[3],
                ]));
                ops.push(ScriptOp::PathSector { radius, radians });
                rest = tail;
            }
            0x30 => {
                if rest.len() < 6 {
                    return Err("circle opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (r_bytes, tail) = tail.split_at(4);
                let radius = f32::from_bits(u32::from_be_bytes([
                    r_bytes[0], r_bytes[1], r_bytes[2], r_bytes[3],
                ]));
                ops.push(ScriptOp::PathCircle { radius });
                rest = tail;
            }
            0x31 => {
                if rest.len() < 10 {
                    return Err("ellipse opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (r0_bytes, tail) = tail.split_at(4);
                let (r1_bytes, tail) = tail.split_at(4);
                let radius0 = f32::from_bits(u32::from_be_bytes([
                    r0_bytes[0],
                    r0_bytes[1],
                    r0_bytes[2],
                    r0_bytes[3],
                ]));
                let radius1 = f32::from_bits(u32::from_be_bytes([
                    r1_bytes[0],
                    r1_bytes[1],
                    r1_bytes[2],
                    r1_bytes[3],
                ]));
                ops.push(ScriptOp::PathEllipse { radius0, radius1 });
                rest = tail;
            }
            0x32 => {
                if rest.len() < 26 {
                    return Err("arc opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (cx_bytes, tail) = tail.split_at(4);
                let (cy_bytes, tail) = tail.split_at(4);
                let (r_bytes, tail) = tail.split_at(4);
                let (a0_bytes, tail) = tail.split_at(4);
                let (a1_bytes, tail) = tail.split_at(4);
                let (dir_bytes, tail) = tail.split_at(4);
                let cx = f32::from_bits(u32::from_be_bytes([
                    cx_bytes[0],
                    cx_bytes[1],
                    cx_bytes[2],
                    cx_bytes[3],
                ]));
                let cy = f32::from_bits(u32::from_be_bytes([
                    cy_bytes[0],
                    cy_bytes[1],
                    cy_bytes[2],
                    cy_bytes[3],
                ]));
                let radius = f32::from_bits(u32::from_be_bytes([
                    r_bytes[0], r_bytes[1], r_bytes[2], r_bytes[3],
                ]));
                let start = f32::from_bits(u32::from_be_bytes([
                    a0_bytes[0],
                    a0_bytes[1],
                    a0_bytes[2],
                    a0_bytes[3],
                ]));
                let end = f32::from_bits(u32::from_be_bytes([
                    a1_bytes[0],
                    a1_bytes[1],
                    a1_bytes[2],
                    a1_bytes[3],
                ]));
                let dir =
                    u32::from_be_bytes([dir_bytes[0], dir_bytes[1], dir_bytes[2], dir_bytes[3]]);
                ops.push(ScriptOp::PathArc {
                    cx,
                    cy,
                    radius,
                    start,
                    end,
                    dir,
                });
                rest = tail;
            }
            0x0f => {
                if rest.len() < 2 {
                    return Err("draw_script opcode truncated".to_string());
                }
                let (len_bytes, tail) = rest.split_at(2);
                let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                let pad = (4 - (len % 4)) % 4;
                let total = len + pad;
                if tail.len() < total {
                    return Err("draw_script payload truncated".to_string());
                }
                let (id_bytes, tail) = tail.split_at(len);
                let id = String::from_utf8_lossy(id_bytes).to_string();
                ops.push(ScriptOp::DrawScript(id));
                rest = &tail[pad..];
            }
            0x40 => {
                if rest.len() < 2 {
                    return Err("push_state opcode truncated".to_string());
                }
                ops.push(ScriptOp::PushState);
                rest = &rest[2..];
            }
            0x41 => {
                if rest.len() < 2 {
                    return Err("pop_state opcode truncated".to_string());
                }
                ops.push(ScriptOp::PopState);
                rest = &rest[2..];
            }
            0x42 => {
                if rest.len() < 2 {
                    return Err("pop_push_state opcode truncated".to_string());
                }
                ops.push(ScriptOp::PopPushState);
                rest = &rest[2..];
            }
            0x60 => {
                if rest.len() < 6 {
                    return Err("fill_color opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (rgba, tail) = tail.split_at(4);
                ops.push(ScriptOp::FillColor(skia_safe::Color::from_argb(
                    rgba[3], rgba[0], rgba[1], rgba[2],
                )));
                rest = tail;
            }
            0x61 => {
                if rest.len() < 26 {
                    return Err("fill_linear opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (start_x_bytes, tail) = tail.split_at(4);
                let (start_y_bytes, tail) = tail.split_at(4);
                let (end_x_bytes, tail) = tail.split_at(4);
                let (end_y_bytes, tail) = tail.split_at(4);
                let (start_rgba, tail) = tail.split_at(4);
                let (end_rgba, tail) = tail.split_at(4);
                let start_x = f32::from_bits(u32::from_be_bytes([
                    start_x_bytes[0],
                    start_x_bytes[1],
                    start_x_bytes[2],
                    start_x_bytes[3],
                ]));
                let start_y = f32::from_bits(u32::from_be_bytes([
                    start_y_bytes[0],
                    start_y_bytes[1],
                    start_y_bytes[2],
                    start_y_bytes[3],
                ]));
                let end_x = f32::from_bits(u32::from_be_bytes([
                    end_x_bytes[0],
                    end_x_bytes[1],
                    end_x_bytes[2],
                    end_x_bytes[3],
                ]));
                let end_y = f32::from_bits(u32::from_be_bytes([
                    end_y_bytes[0],
                    end_y_bytes[1],
                    end_y_bytes[2],
                    end_y_bytes[3],
                ]));
                let start_color = skia_safe::Color::from_argb(
                    start_rgba[3],
                    start_rgba[0],
                    start_rgba[1],
                    start_rgba[2],
                );
                let end_color =
                    skia_safe::Color::from_argb(end_rgba[3], end_rgba[0], end_rgba[1], end_rgba[2]);
                ops.push(ScriptOp::FillLinear {
                    start_x,
                    start_y,
                    end_x,
                    end_y,
                    start_color,
                    end_color,
                });
                rest = tail;
            }
            0x62 => {
                if rest.len() < 26 {
                    return Err("fill_radial opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (center_x_bytes, tail) = tail.split_at(4);
                let (center_y_bytes, tail) = tail.split_at(4);
                let (inner_bytes, tail) = tail.split_at(4);
                let (outer_bytes, tail) = tail.split_at(4);
                let (start_rgba, tail) = tail.split_at(4);
                let (end_rgba, tail) = tail.split_at(4);
                let center_x = f32::from_bits(u32::from_be_bytes([
                    center_x_bytes[0],
                    center_x_bytes[1],
                    center_x_bytes[2],
                    center_x_bytes[3],
                ]));
                let center_y = f32::from_bits(u32::from_be_bytes([
                    center_y_bytes[0],
                    center_y_bytes[1],
                    center_y_bytes[2],
                    center_y_bytes[3],
                ]));
                let inner_radius = f32::from_bits(u32::from_be_bytes([
                    inner_bytes[0],
                    inner_bytes[1],
                    inner_bytes[2],
                    inner_bytes[3],
                ]));
                let outer_radius = f32::from_bits(u32::from_be_bytes([
                    outer_bytes[0],
                    outer_bytes[1],
                    outer_bytes[2],
                    outer_bytes[3],
                ]));
                let start_color = skia_safe::Color::from_argb(
                    start_rgba[3],
                    start_rgba[0],
                    start_rgba[1],
                    start_rgba[2],
                );
                let end_color =
                    skia_safe::Color::from_argb(end_rgba[3], end_rgba[0], end_rgba[1], end_rgba[2]);
                ops.push(ScriptOp::FillRadial {
                    center_x,
                    center_y,
                    inner_radius,
                    outer_radius,
                    start_color,
                    end_color,
                });
                rest = tail;
            }
            0x63 => {
                if rest.len() < 2 {
                    return Err("fill_image opcode truncated".to_string());
                }
                let (len_bytes, tail) = rest.split_at(2);
                let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                let pad = (4 - (len % 4)) % 4;
                let total = len + pad;
                if tail.len() < total {
                    return Err("fill_image payload truncated".to_string());
                }
                let (id_bytes, tail) = tail.split_at(len);
                let id = String::from_utf8_lossy(id_bytes).to_string();
                ops.push(ScriptOp::FillImage(id));
                rest = &tail[pad..];
            }
            0x64 => {
                if rest.len() < 2 {
                    return Err("fill_stream opcode truncated".to_string());
                }
                let (len_bytes, tail) = rest.split_at(2);
                let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                let pad = (4 - (len % 4)) % 4;
                let total = len + pad;
                if tail.len() < total {
                    return Err("fill_stream payload truncated".to_string());
                }
                let (id_bytes, tail) = tail.split_at(len);
                let id = String::from_utf8_lossy(id_bytes).to_string();
                ops.push(ScriptOp::FillStream(id));
                rest = &tail[pad..];
            }
            0x50 => {
                if rest.len() < 26 {
                    return Err("transform opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (a_bytes, tail) = tail.split_at(4);
                let (b_bytes, tail) = tail.split_at(4);
                let (c_bytes, tail) = tail.split_at(4);
                let (d_bytes, tail) = tail.split_at(4);
                let (e_bytes, tail) = tail.split_at(4);
                let (f_bytes, tail) = tail.split_at(4);
                let a = f32::from_bits(u32::from_be_bytes([
                    a_bytes[0], a_bytes[1], a_bytes[2], a_bytes[3],
                ]));
                let b = f32::from_bits(u32::from_be_bytes([
                    b_bytes[0], b_bytes[1], b_bytes[2], b_bytes[3],
                ]));
                let c = f32::from_bits(u32::from_be_bytes([
                    c_bytes[0], c_bytes[1], c_bytes[2], c_bytes[3],
                ]));
                let d = f32::from_bits(u32::from_be_bytes([
                    d_bytes[0], d_bytes[1], d_bytes[2], d_bytes[3],
                ]));
                let e = f32::from_bits(u32::from_be_bytes([
                    e_bytes[0], e_bytes[1], e_bytes[2], e_bytes[3],
                ]));
                let f = f32::from_bits(u32::from_be_bytes([
                    f_bytes[0], f_bytes[1], f_bytes[2], f_bytes[3],
                ]));
                ops.push(ScriptOp::Transform { a, b, c, d, e, f });
                rest = tail;
            }
            0x51 => {
                if rest.len() < 10 {
                    return Err("scale opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (x_bytes, tail) = tail.split_at(4);
                let (y_bytes, tail) = tail.split_at(4);
                let x = f32::from_bits(u32::from_be_bytes([
                    x_bytes[0], x_bytes[1], x_bytes[2], x_bytes[3],
                ]));
                let y = f32::from_bits(u32::from_be_bytes([
                    y_bytes[0], y_bytes[1], y_bytes[2], y_bytes[3],
                ]));
                ops.push(ScriptOp::Scale(x, y));
                rest = tail;
            }
            0x52 => {
                if rest.len() < 6 {
                    return Err("rotate opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (r_bytes, tail) = tail.split_at(4);
                let radians = f32::from_bits(u32::from_be_bytes([
                    r_bytes[0], r_bytes[1], r_bytes[2], r_bytes[3],
                ]));
                ops.push(ScriptOp::Rotate(radians));
                rest = tail;
            }
            0x53 => {
                if rest.len() < 10 {
                    return Err("translate opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (x_bytes, tail) = tail.split_at(4);
                let (y_bytes, tail) = tail.split_at(4);
                let x = f32::from_bits(u32::from_be_bytes([
                    x_bytes[0], x_bytes[1], x_bytes[2], x_bytes[3],
                ]));
                let y = f32::from_bits(u32::from_be_bytes([
                    y_bytes[0], y_bytes[1], y_bytes[2], y_bytes[3],
                ]));
                ops.push(ScriptOp::Translate(x, y));
                rest = tail;
            }
            0x01 => {
                if rest.len() < 18 {
                    return Err("draw_line opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (x0_bytes, tail) = tail.split_at(4);
                let (y0_bytes, tail) = tail.split_at(4);
                let (x1_bytes, tail) = tail.split_at(4);
                let (y1_bytes, tail) = tail.split_at(4);
                let x0 = f32::from_bits(u32::from_be_bytes([
                    x0_bytes[0],
                    x0_bytes[1],
                    x0_bytes[2],
                    x0_bytes[3],
                ]));
                let y0 = f32::from_bits(u32::from_be_bytes([
                    y0_bytes[0],
                    y0_bytes[1],
                    y0_bytes[2],
                    y0_bytes[3],
                ]));
                let x1 = f32::from_bits(u32::from_be_bytes([
                    x1_bytes[0],
                    x1_bytes[1],
                    x1_bytes[2],
                    x1_bytes[3],
                ]));
                let y1 = f32::from_bits(u32::from_be_bytes([
                    y1_bytes[0],
                    y1_bytes[1],
                    y1_bytes[2],
                    y1_bytes[3],
                ]));
                ops.push(ScriptOp::DrawLine {
                    x0,
                    y0,
                    x1,
                    y1,
                    flag,
                });
                rest = tail;
            }
            0x02 => {
                if rest.len() < 26 {
                    return Err("draw_triangle opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (x0_bytes, tail) = tail.split_at(4);
                let (y0_bytes, tail) = tail.split_at(4);
                let (x1_bytes, tail) = tail.split_at(4);
                let (y1_bytes, tail) = tail.split_at(4);
                let (x2_bytes, tail) = tail.split_at(4);
                let (y2_bytes, tail) = tail.split_at(4);
                let x0 = f32::from_bits(u32::from_be_bytes([
                    x0_bytes[0],
                    x0_bytes[1],
                    x0_bytes[2],
                    x0_bytes[3],
                ]));
                let y0 = f32::from_bits(u32::from_be_bytes([
                    y0_bytes[0],
                    y0_bytes[1],
                    y0_bytes[2],
                    y0_bytes[3],
                ]));
                let x1 = f32::from_bits(u32::from_be_bytes([
                    x1_bytes[0],
                    x1_bytes[1],
                    x1_bytes[2],
                    x1_bytes[3],
                ]));
                let y1 = f32::from_bits(u32::from_be_bytes([
                    y1_bytes[0],
                    y1_bytes[1],
                    y1_bytes[2],
                    y1_bytes[3],
                ]));
                let x2 = f32::from_bits(u32::from_be_bytes([
                    x2_bytes[0],
                    x2_bytes[1],
                    x2_bytes[2],
                    x2_bytes[3],
                ]));
                let y2 = f32::from_bits(u32::from_be_bytes([
                    y2_bytes[0],
                    y2_bytes[1],
                    y2_bytes[2],
                    y2_bytes[3],
                ]));
                ops.push(ScriptOp::DrawTriangle {
                    x0,
                    y0,
                    x1,
                    y1,
                    x2,
                    y2,
                    flag,
                });
                rest = tail;
            }
            0x03 => {
                if rest.len() < 34 {
                    return Err("draw_quad opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (x0_bytes, tail) = tail.split_at(4);
                let (y0_bytes, tail) = tail.split_at(4);
                let (x1_bytes, tail) = tail.split_at(4);
                let (y1_bytes, tail) = tail.split_at(4);
                let (x2_bytes, tail) = tail.split_at(4);
                let (y2_bytes, tail) = tail.split_at(4);
                let (x3_bytes, tail) = tail.split_at(4);
                let (y3_bytes, tail) = tail.split_at(4);
                let x0 = f32::from_bits(u32::from_be_bytes([
                    x0_bytes[0],
                    x0_bytes[1],
                    x0_bytes[2],
                    x0_bytes[3],
                ]));
                let y0 = f32::from_bits(u32::from_be_bytes([
                    y0_bytes[0],
                    y0_bytes[1],
                    y0_bytes[2],
                    y0_bytes[3],
                ]));
                let x1 = f32::from_bits(u32::from_be_bytes([
                    x1_bytes[0],
                    x1_bytes[1],
                    x1_bytes[2],
                    x1_bytes[3],
                ]));
                let y1 = f32::from_bits(u32::from_be_bytes([
                    y1_bytes[0],
                    y1_bytes[1],
                    y1_bytes[2],
                    y1_bytes[3],
                ]));
                let x2 = f32::from_bits(u32::from_be_bytes([
                    x2_bytes[0],
                    x2_bytes[1],
                    x2_bytes[2],
                    x2_bytes[3],
                ]));
                let y2 = f32::from_bits(u32::from_be_bytes([
                    y2_bytes[0],
                    y2_bytes[1],
                    y2_bytes[2],
                    y2_bytes[3],
                ]));
                let x3 = f32::from_bits(u32::from_be_bytes([
                    x3_bytes[0],
                    x3_bytes[1],
                    x3_bytes[2],
                    x3_bytes[3],
                ]));
                let y3 = f32::from_bits(u32::from_be_bytes([
                    y3_bytes[0],
                    y3_bytes[1],
                    y3_bytes[2],
                    y3_bytes[3],
                ]));
                ops.push(ScriptOp::DrawQuad {
                    x0,
                    y0,
                    x1,
                    y1,
                    x2,
                    y2,
                    x3,
                    y3,
                    flag,
                });
                rest = tail;
            }
            0x04 => {
                if rest.len() < 10 {
                    return Err("draw_rect opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (w_bytes, tail) = tail.split_at(4);
                let (h_bytes, tail) = tail.split_at(4);
                let width = f32::from_bits(u32::from_be_bytes([
                    w_bytes[0], w_bytes[1], w_bytes[2], w_bytes[3],
                ]));
                let height = f32::from_bits(u32::from_be_bytes([
                    h_bytes[0], h_bytes[1], h_bytes[2], h_bytes[3],
                ]));
                ops.push(ScriptOp::DrawRect {
                    width,
                    height,
                    flag,
                });
                rest = tail;
            }
            0x05 => {
                if rest.len() < 14 {
                    return Err("draw_rrect opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (w_bytes, tail) = tail.split_at(4);
                let (h_bytes, tail) = tail.split_at(4);
                let (r_bytes, tail) = tail.split_at(4);
                let width = f32::from_bits(u32::from_be_bytes([
                    w_bytes[0], w_bytes[1], w_bytes[2], w_bytes[3],
                ]));
                let height = f32::from_bits(u32::from_be_bytes([
                    h_bytes[0], h_bytes[1], h_bytes[2], h_bytes[3],
                ]));
                let radius = f32::from_bits(u32::from_be_bytes([
                    r_bytes[0], r_bytes[1], r_bytes[2], r_bytes[3],
                ]));
                ops.push(ScriptOp::DrawRRect {
                    width,
                    height,
                    radius,
                    flag,
                });
                rest = tail;
            }
            0x0C => {
                if rest.len() < 26 {
                    return Err("draw_rrectv opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (w_bytes, tail) = tail.split_at(4);
                let (h_bytes, tail) = tail.split_at(4);
                let (ul_bytes, tail) = tail.split_at(4);
                let (ur_bytes, tail) = tail.split_at(4);
                let (lr_bytes, tail) = tail.split_at(4);
                let (ll_bytes, tail) = tail.split_at(4);
                let width = f32::from_bits(u32::from_be_bytes([
                    w_bytes[0], w_bytes[1], w_bytes[2], w_bytes[3],
                ]));
                let height = f32::from_bits(u32::from_be_bytes([
                    h_bytes[0], h_bytes[1], h_bytes[2], h_bytes[3],
                ]));
                let ul_radius = f32::from_bits(u32::from_be_bytes([
                    ul_bytes[0],
                    ul_bytes[1],
                    ul_bytes[2],
                    ul_bytes[3],
                ]));
                let ur_radius = f32::from_bits(u32::from_be_bytes([
                    ur_bytes[0],
                    ur_bytes[1],
                    ur_bytes[2],
                    ur_bytes[3],
                ]));
                let lr_radius = f32::from_bits(u32::from_be_bytes([
                    lr_bytes[0],
                    lr_bytes[1],
                    lr_bytes[2],
                    lr_bytes[3],
                ]));
                let ll_radius = f32::from_bits(u32::from_be_bytes([
                    ll_bytes[0],
                    ll_bytes[1],
                    ll_bytes[2],
                    ll_bytes[3],
                ]));
                ops.push(ScriptOp::DrawRRectV {
                    width,
                    height,
                    ul_radius,
                    ur_radius,
                    lr_radius,
                    ll_radius,
                    flag,
                });
                rest = tail;
            }
            0x06 => {
                if rest.len() < 10 {
                    return Err("draw_arc opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (radius_bytes, tail) = tail.split_at(4);
                let (radians_bytes, tail) = tail.split_at(4);
                let radius = f32::from_bits(u32::from_be_bytes([
                    radius_bytes[0],
                    radius_bytes[1],
                    radius_bytes[2],
                    radius_bytes[3],
                ]));
                let radians = f32::from_bits(u32::from_be_bytes([
                    radians_bytes[0],
                    radians_bytes[1],
                    radians_bytes[2],
                    radians_bytes[3],
                ]));
                ops.push(ScriptOp::DrawArc {
                    radius,
                    radians,
                    flag,
                });
                rest = tail;
            }
            0x07 => {
                if rest.len() < 10 {
                    return Err("draw_sector opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (radius_bytes, tail) = tail.split_at(4);
                let (radians_bytes, tail) = tail.split_at(4);
                let radius = f32::from_bits(u32::from_be_bytes([
                    radius_bytes[0],
                    radius_bytes[1],
                    radius_bytes[2],
                    radius_bytes[3],
                ]));
                let radians = f32::from_bits(u32::from_be_bytes([
                    radians_bytes[0],
                    radians_bytes[1],
                    radians_bytes[2],
                    radians_bytes[3],
                ]));
                ops.push(ScriptOp::DrawSector {
                    radius,
                    radians,
                    flag,
                });
                rest = tail;
            }
            0x08 => {
                if rest.len() < 6 {
                    return Err("draw_circle opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (r_bytes, tail) = tail.split_at(4);
                let radius = f32::from_bits(u32::from_be_bytes([
                    r_bytes[0], r_bytes[1], r_bytes[2], r_bytes[3],
                ]));
                ops.push(ScriptOp::DrawCircle { radius, flag });
                rest = tail;
            }
            0x09 => {
                if rest.len() < 10 {
                    return Err("draw_ellipse opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (r0_bytes, tail) = tail.split_at(4);
                let (r1_bytes, tail) = tail.split_at(4);
                let radius0 = f32::from_bits(u32::from_be_bytes([
                    r0_bytes[0],
                    r0_bytes[1],
                    r0_bytes[2],
                    r0_bytes[3],
                ]));
                let radius1 = f32::from_bits(u32::from_be_bytes([
                    r1_bytes[0],
                    r1_bytes[1],
                    r1_bytes[2],
                    r1_bytes[3],
                ]));
                ops.push(ScriptOp::DrawEllipse {
                    radius0,
                    radius1,
                    flag,
                });
                rest = tail;
            }
            0x0B => {
                if rest.len() < 6 {
                    return Err("draw_sprites opcode truncated".to_string());
                }
                let (len_bytes, tail) = rest.split_at(2);
                let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                let (count_bytes, tail) = tail.split_at(4);
                let count = u32::from_be_bytes([
                    count_bytes[0],
                    count_bytes[1],
                    count_bytes[2],
                    count_bytes[3],
                ]) as usize;
                let pad = (4 - (len % 4)) % 4;
                let total = len + pad;
                if tail.len() < total {
                    return Err("draw_sprites payload truncated".to_string());
                }
                let (id_bytes, tail) = tail.split_at(len);
                let id = String::from_utf8_lossy(id_bytes).to_string();
                let tail = &tail[pad..];
                let cmd_bytes = count
                    .checked_mul(9)
                    .and_then(|v| v.checked_mul(4))
                    .ok_or_else(|| "draw_sprites command overflow".to_string())?;
                if tail.len() < cmd_bytes {
                    return Err("draw_sprites command data truncated".to_string());
                }
                let (cmds_bytes, tail) = tail.split_at(cmd_bytes);
                let mut cmds = Vec::with_capacity(count);
                let mut cmd_rest = cmds_bytes;
                for _ in 0..count {
                    let (cmd, next) = cmd_rest.split_at(36);
                    let sx = f32::from_bits(u32::from_be_bytes([cmd[0], cmd[1], cmd[2], cmd[3]]));
                    let sy = f32::from_bits(u32::from_be_bytes([cmd[4], cmd[5], cmd[6], cmd[7]]));
                    let sw = f32::from_bits(u32::from_be_bytes([cmd[8], cmd[9], cmd[10], cmd[11]]));
                    let sh =
                        f32::from_bits(u32::from_be_bytes([cmd[12], cmd[13], cmd[14], cmd[15]]));
                    let dx =
                        f32::from_bits(u32::from_be_bytes([cmd[16], cmd[17], cmd[18], cmd[19]]));
                    let dy =
                        f32::from_bits(u32::from_be_bytes([cmd[20], cmd[21], cmd[22], cmd[23]]));
                    let dw =
                        f32::from_bits(u32::from_be_bytes([cmd[24], cmd[25], cmd[26], cmd[27]]));
                    let dh =
                        f32::from_bits(u32::from_be_bytes([cmd[28], cmd[29], cmd[30], cmd[31]]));
                    let alpha =
                        f32::from_bits(u32::from_be_bytes([cmd[32], cmd[33], cmd[34], cmd[35]]));
                    cmds.push(crate::renderer::SpriteCommand {
                        sx,
                        sy,
                        sw,
                        sh,
                        dx,
                        dy,
                        dw,
                        dh,
                        alpha,
                    });
                    cmd_rest = next;
                }
                ops.push(ScriptOp::DrawSprites { image_id: id, cmds });
                rest = tail;
            }
            0x0A => {
                if rest.len() < 2 {
                    return Err("draw_text opcode truncated".to_string());
                }
                let (len_bytes, tail) = rest.split_at(2);
                let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                let pad = (4 - (len % 4)) % 4;
                let total = len + pad;
                if tail.len() < total {
                    return Err("draw_text payload truncated".to_string());
                }
                let (text_bytes, tail) = tail.split_at(len);
                let text = String::from_utf8_lossy(text_bytes).to_string();
                ops.push(ScriptOp::DrawText(text));
                rest = &tail[pad..];
            }
            0x70 => {
                if rest.len() < 2 {
                    return Err("stroke_width opcode truncated".to_string());
                }
                let (width_bytes, tail) = rest.split_at(2);
                let width = u16::from_be_bytes([width_bytes[0], width_bytes[1]]);
                ops.push(ScriptOp::StrokeWidth(width as f32 / 4.0));
                rest = tail;
            }
            0x71 => {
                if rest.len() < 6 {
                    return Err("stroke_color opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (rgba, tail) = tail.split_at(4);
                ops.push(ScriptOp::StrokeColor(skia_safe::Color::from_argb(
                    rgba[3], rgba[0], rgba[1], rgba[2],
                )));
                rest = tail;
            }
            0x72 => {
                if rest.len() < 26 {
                    return Err("stroke_linear opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (start_x_bytes, tail) = tail.split_at(4);
                let (start_y_bytes, tail) = tail.split_at(4);
                let (end_x_bytes, tail) = tail.split_at(4);
                let (end_y_bytes, tail) = tail.split_at(4);
                let (start_rgba, tail) = tail.split_at(4);
                let (end_rgba, tail) = tail.split_at(4);
                let start_x = f32::from_bits(u32::from_be_bytes([
                    start_x_bytes[0],
                    start_x_bytes[1],
                    start_x_bytes[2],
                    start_x_bytes[3],
                ]));
                let start_y = f32::from_bits(u32::from_be_bytes([
                    start_y_bytes[0],
                    start_y_bytes[1],
                    start_y_bytes[2],
                    start_y_bytes[3],
                ]));
                let end_x = f32::from_bits(u32::from_be_bytes([
                    end_x_bytes[0],
                    end_x_bytes[1],
                    end_x_bytes[2],
                    end_x_bytes[3],
                ]));
                let end_y = f32::from_bits(u32::from_be_bytes([
                    end_y_bytes[0],
                    end_y_bytes[1],
                    end_y_bytes[2],
                    end_y_bytes[3],
                ]));
                let start_color = skia_safe::Color::from_argb(
                    start_rgba[3],
                    start_rgba[0],
                    start_rgba[1],
                    start_rgba[2],
                );
                let end_color =
                    skia_safe::Color::from_argb(end_rgba[3], end_rgba[0], end_rgba[1], end_rgba[2]);
                ops.push(ScriptOp::StrokeLinear {
                    start_x,
                    start_y,
                    end_x,
                    end_y,
                    start_color,
                    end_color,
                });
                rest = tail;
            }
            0x73 => {
                if rest.len() < 26 {
                    return Err("stroke_radial opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (center_x_bytes, tail) = tail.split_at(4);
                let (center_y_bytes, tail) = tail.split_at(4);
                let (inner_bytes, tail) = tail.split_at(4);
                let (outer_bytes, tail) = tail.split_at(4);
                let (start_rgba, tail) = tail.split_at(4);
                let (end_rgba, tail) = tail.split_at(4);
                let center_x = f32::from_bits(u32::from_be_bytes([
                    center_x_bytes[0],
                    center_x_bytes[1],
                    center_x_bytes[2],
                    center_x_bytes[3],
                ]));
                let center_y = f32::from_bits(u32::from_be_bytes([
                    center_y_bytes[0],
                    center_y_bytes[1],
                    center_y_bytes[2],
                    center_y_bytes[3],
                ]));
                let inner_radius = f32::from_bits(u32::from_be_bytes([
                    inner_bytes[0],
                    inner_bytes[1],
                    inner_bytes[2],
                    inner_bytes[3],
                ]));
                let outer_radius = f32::from_bits(u32::from_be_bytes([
                    outer_bytes[0],
                    outer_bytes[1],
                    outer_bytes[2],
                    outer_bytes[3],
                ]));
                let start_color = skia_safe::Color::from_argb(
                    start_rgba[3],
                    start_rgba[0],
                    start_rgba[1],
                    start_rgba[2],
                );
                let end_color =
                    skia_safe::Color::from_argb(end_rgba[3], end_rgba[0], end_rgba[1], end_rgba[2]);
                ops.push(ScriptOp::StrokeRadial {
                    center_x,
                    center_y,
                    inner_radius,
                    outer_radius,
                    start_color,
                    end_color,
                });
                rest = tail;
            }
            0x74 => {
                if rest.len() < 2 {
                    return Err("stroke_image opcode truncated".to_string());
                }
                let (len_bytes, tail) = rest.split_at(2);
                let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                let pad = (4 - (len % 4)) % 4;
                let total = len + pad;
                if tail.len() < total {
                    return Err("stroke_image payload truncated".to_string());
                }
                let (id_bytes, tail) = tail.split_at(len);
                let id = String::from_utf8_lossy(id_bytes).to_string();
                ops.push(ScriptOp::StrokeImage(id));
                rest = &tail[pad..];
            }
            0x75 => {
                if rest.len() < 2 {
                    return Err("stroke_stream opcode truncated".to_string());
                }
                let (len_bytes, tail) = rest.split_at(2);
                let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                let pad = (4 - (len % 4)) % 4;
                let total = len + pad;
                if tail.len() < total {
                    return Err("stroke_stream payload truncated".to_string());
                }
                let (id_bytes, tail) = tail.split_at(len);
                let id = String::from_utf8_lossy(id_bytes).to_string();
                ops.push(ScriptOp::StrokeStream(id));
                rest = &tail[pad..];
            }
            0x80 => {
                if rest.len() < 2 {
                    return Err("cap opcode truncated".to_string());
                }
                let (cap_bytes, tail) = rest.split_at(2);
                let cap = u16::from_be_bytes([cap_bytes[0], cap_bytes[1]]);
                let cap = match cap {
                    0x00 => skia_safe::PaintCap::Butt,
                    0x01 => skia_safe::PaintCap::Round,
                    0x02 => skia_safe::PaintCap::Square,
                    _ => return Err("cap opcode invalid".to_string()),
                };
                ops.push(ScriptOp::StrokeCap(cap));
                rest = tail;
            }
            0x81 => {
                if rest.len() < 2 {
                    return Err("join opcode truncated".to_string());
                }
                let (join_bytes, tail) = rest.split_at(2);
                let join = u16::from_be_bytes([join_bytes[0], join_bytes[1]]);
                let join = match join {
                    0x00 => skia_safe::PaintJoin::Bevel,
                    0x01 => skia_safe::PaintJoin::Round,
                    0x02 => skia_safe::PaintJoin::Miter,
                    _ => return Err("join opcode invalid".to_string()),
                };
                ops.push(ScriptOp::StrokeJoin(join));
                rest = tail;
            }
            0x82 => {
                if rest.len() < 2 {
                    return Err("miter_limit opcode truncated".to_string());
                }
                let (limit_bytes, tail) = rest.split_at(2);
                let limit = u16::from_be_bytes([limit_bytes[0], limit_bytes[1]]);
                ops.push(ScriptOp::StrokeMiterLimit(limit as f32));
                rest = tail;
            }
            0x90 => {
                if rest.len() < 2 {
                    return Err("font opcode truncated".to_string());
                }
                let (len_bytes, tail) = rest.split_at(2);
                let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                let pad = (4 - (len % 4)) % 4;
                let total = len + pad;
                if tail.len() < total {
                    return Err("font payload truncated".to_string());
                }
                let (font_bytes, tail) = tail.split_at(len);
                let font_id = String::from_utf8_lossy(font_bytes).to_string();
                ops.push(ScriptOp::Font(font_id));
                rest = &tail[pad..];
            }
            0x91 => {
                if rest.len() < 2 {
                    return Err("font_size opcode truncated".to_string());
                }
                let (size_bytes, tail) = rest.split_at(2);
                let size = u16::from_be_bytes([size_bytes[0], size_bytes[1]]);
                ops.push(ScriptOp::FontSize(size as f32 / 4.0));
                rest = tail;
            }
            0x92 => {
                if rest.len() < 2 {
                    return Err("text_align opcode truncated".to_string());
                }
                let (align_bytes, tail) = rest.split_at(2);
                let align = u16::from_be_bytes([align_bytes[0], align_bytes[1]]);
                let align = match align {
                    0x00 => renderer::TextAlign::Left,
                    0x01 => renderer::TextAlign::Center,
                    0x02 => renderer::TextAlign::Right,
                    _ => return Err("unsupported text_align value".to_string()),
                };
                ops.push(ScriptOp::TextAlign(align));
                rest = tail;
            }
            0x93 => {
                if rest.len() < 2 {
                    return Err("text_base opcode truncated".to_string());
                }
                let (base_bytes, tail) = rest.split_at(2);
                let base = u16::from_be_bytes([base_bytes[0], base_bytes[1]]);
                let base = match base {
                    0x00 => renderer::TextBase::Top,
                    0x01 => renderer::TextBase::Middle,
                    0x02 => renderer::TextBase::Alphabetic,
                    0x03 => renderer::TextBase::Bottom,
                    _ => return Err("unsupported text_base value".to_string()),
                };
                ops.push(ScriptOp::TextBase(base));
                rest = tail;
            }
            _ => {
                return Err(format!("unsupported opcode: 0x{opcode:02x}"));
            }
        }
    }
    Ok(ops)
}

fn load(env: Env, _info: Term) -> bool {
    env.register::<RendererResource>().is_ok()
}

rustler::init!("Elixir.Scenic.Driver.Skia.Native", load = load);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputEvent, InputQueue};
    use crate::renderer::SpriteCommand;

    #[test]
    fn parse_fill_and_rect() {
        let script: [u8; 20] = [
            0x00, 0x60, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x04, 0x00, 0x01, 0x42, 0x20,
            0x00, 0x00, 0x41, 0xA0, 0x00, 0x00,
        ];
        let ops = parse_script(&script).expect("parse_script failed");

        assert_eq!(
            ops,
            vec![
                ScriptOp::FillColor(skia_safe::Color::from_argb(0xFF, 0xFF, 0x00, 0x00)),
                ScriptOp::DrawRect {
                    width: 40.0,
                    height: 20.0,
                    flag: 0x01,
                }
            ]
        );
    }

    #[test]
    fn parse_rejects_truncated_fill_color() {
        let script: [u8; 4] = [0x00, 0x60, 0x00, 0x00];
        let err = parse_script(&script).unwrap_err();
        assert!(err.contains("fill_color opcode truncated"));
    }

    #[test]
    fn parse_rejects_truncated_rect() {
        let script: [u8; 6] = [0x00, 0x04, 0x00, 0x01, 0x00, 0x00];
        let err = parse_script(&script).unwrap_err();
        assert!(err.contains("draw_rect opcode truncated"));
    }

    #[test]
    fn parse_rejects_unknown_opcode() {
        let script: [u8; 2] = [0x12, 0x34];
        let err = parse_script(&script).unwrap_err();
        assert!(err.contains("unsupported opcode"));
    }

    #[test]
    fn parse_translate_affects_rect() {
        let script: [u8; 40] = [
            0x00, 0x40, 0x00, 0x00, 0x00, 0x53, 0x00, 0x00, 0x42, 0x48, 0x00, 0x00, 0x42, 0x70,
            0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0x04, 0x00, 0x01,
            0x41, 0x20, 0x00, 0x00, 0x41, 0xA0, 0x00, 0x00, 0x00, 0x41, 0x00, 0x00,
        ];
        let ops = parse_script(&script).expect("parse_script failed");

        assert!(ops.contains(&ScriptOp::Translate(50.0, 60.0)));
        assert!(ops.contains(&ScriptOp::DrawRect {
            width: 10.0,
            height: 20.0,
            flag: 0x01
        }));
    }

    #[test]
    fn parse_includes_draw_script() {
        let mut script: Vec<u8> = vec![0x00, 0x0f, 0x00, 0x04];
        script.extend_from_slice(b"root");
        script.extend_from_slice(&[
            0x00, 0x60, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x04, 0x00, 0x01, 0x41, 0x20,
            0x00, 0x00, 0x41, 0xA0, 0x00, 0x00,
        ]);
        let ops = parse_script(&script).expect("parse_script failed");
        assert!(ops.contains(&ScriptOp::DrawScript("root".to_string())));
    }

    #[test]
    fn parse_draw_text() {
        let script: [u8; 8] = [0x00, 0x0A, 0x00, 0x02, b'h', b'i', 0x00, 0x00];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(ops, vec![ScriptOp::DrawText("hi".to_string())]);
    }

    #[test]
    fn parse_draw_sprites() {
        let mut script: Vec<u8> = Vec::new();
        script.extend_from_slice(&[0x00, 0x0B, 0x00, 0x06]);
        script.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        script.extend_from_slice(b"sprite");
        script.extend_from_slice(&[0x00, 0x00]);
        push_f32(&mut script, 1.0);
        push_f32(&mut script, 2.0);
        push_f32(&mut script, 3.0);
        push_f32(&mut script, 4.0);
        push_f32(&mut script, 5.0);
        push_f32(&mut script, 6.0);
        push_f32(&mut script, 7.0);
        push_f32(&mut script, 8.0);
        push_f32(&mut script, 0.5);

        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![ScriptOp::DrawSprites {
                image_id: "sprite".to_string(),
                cmds: vec![SpriteCommand {
                    sx: 1.0,
                    sy: 2.0,
                    sw: 3.0,
                    sh: 4.0,
                    dx: 5.0,
                    dy: 6.0,
                    dw: 7.0,
                    dh: 8.0,
                    alpha: 0.5,
                }]
            }]
        );
    }

    #[test]
    fn parse_clip_path() {
        let script: [u8; 4] = [0x00, 0x45, 0x00, 0x00];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(ops, vec![ScriptOp::ClipPath(ClipOp::Intersect)]);
    }

    #[test]
    fn parse_draw_line_and_stroke() {
        let script: [u8; 32] = [
            0x00, 0x70, 0x00, 0x08, 0x00, 0x71, 0x00, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0x01,
            0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x20, 0x00, 0x00,
            0x41, 0xA0, 0x00, 0x00,
        ];
        let ops = parse_script(&script).expect("parse_script failed");
        assert!(ops.contains(&ScriptOp::StrokeWidth(2.0)));
        assert!(
            ops.contains(&ScriptOp::StrokeColor(skia_safe::Color::from_argb(
                0xFF, 0x00, 0xFF, 0x00
            )))
        );
        assert!(ops.contains(&ScriptOp::DrawLine {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 20.0,
            flag: 0x02
        }));
    }

    #[test]
    fn parse_draw_triangle() {
        let script: [u8; 28] = [
            0x00, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x20,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x20, 0x00, 0x00, 0x41, 0xA0, 0x00, 0x00,
        ];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![ScriptOp::DrawTriangle {
                x0: 0.0,
                y0: 0.0,
                x1: 10.0,
                y1: 0.0,
                x2: 10.0,
                y2: 20.0,
                flag: 0x03
            }]
        );
    }

    #[test]
    fn parse_draw_quad() {
        let script: [u8; 36] = [
            0x00, 0x03, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x20,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x20, 0x00, 0x00, 0x41, 0xA0, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x41, 0xA0, 0x00, 0x00,
        ];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![ScriptOp::DrawQuad {
                x0: 0.0,
                y0: 0.0,
                x1: 10.0,
                y1: 0.0,
                x2: 10.0,
                y2: 20.0,
                x3: 0.0,
                y3: 20.0,
                flag: 0x03
            }]
        );
    }

    #[test]
    fn parse_draw_circle() {
        let script: [u8; 8] = [0x00, 0x08, 0x00, 0x03, 0x42, 0x48, 0x00, 0x00];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![ScriptOp::DrawCircle {
                radius: 50.0,
                flag: 0x03
            }]
        );
    }

    #[test]
    fn parse_draw_arc() {
        let script: [u8; 12] = [
            0x00, 0x06, 0x00, 0x03, 0x42, 0x48, 0x00, 0x00, 0x3F, 0xC9, 0x0F, 0xDB,
        ];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![ScriptOp::DrawArc {
                radius: 50.0,
                radians: 1.5707964,
                flag: 0x03
            }]
        );
    }

    #[test]
    fn parse_draw_sector() {
        let script: [u8; 12] = [
            0x00, 0x07, 0x00, 0x03, 0x42, 0x48, 0x00, 0x00, 0x3F, 0xC9, 0x0F, 0xDB,
        ];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![ScriptOp::DrawSector {
                radius: 50.0,
                radians: 1.5707964,
                flag: 0x03
            }]
        );
    }

    #[test]
    fn parse_draw_ellipse() {
        let script: [u8; 12] = [
            0x00, 0x09, 0x00, 0x03, 0x42, 0x48, 0x00, 0x00, 0x41, 0xC8, 0x00, 0x00,
        ];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![ScriptOp::DrawEllipse {
                radius0: 50.0,
                radius1: 25.0,
                flag: 0x03
            }]
        );
    }

    #[test]
    fn drain_input_events_returns_queued_events() {
        let stop = Arc::new(AtomicBool::new(false));
        let thread = thread::spawn(|| {});
        let mut queue = InputQueue::new();
        queue.push_event(InputEvent::CursorPos { x: 1.0, y: 2.0 });
        queue.push_event(InputEvent::Key {
            key: "key_a".to_string(),
            action: 1,
            mods: 0,
        });
        queue.push_event(InputEvent::ViewportReshape {
            width: 1280,
            height: 720,
        });
        let input_events = Arc::new(Mutex::new(queue));

        let handle = DriverHandle {
            stop: StopSignal::Raster(Arc::clone(&stop)),
            text: Arc::new(Mutex::new(String::new())),
            render_state: Arc::new(Mutex::new(RenderState::default())),
            input_events: Arc::clone(&input_events),
            input_mask: Arc::new(AtomicU32::new(0)),
            raster_frame: None,
            dirty: Some(Arc::new(AtomicBool::new(false))),
            running: Arc::new(AtomicBool::new(false)),
            cursor_state: None,
            thread: Some(thread),
        };
        let renderer = RendererResource {
            handle: Mutex::new(handle),
        };

        let drained = drain_input_events_inner(&renderer).expect("drain_input_events failed");
        assert_eq!(drained.len(), 3);
        assert!(matches!(drained[0], InputEvent::CursorPos { .. }));
        assert!(matches!(drained[1], InputEvent::Key { .. }));
        assert!(matches!(drained[2], InputEvent::ViewportReshape { .. }));
    }

    #[test]
    fn parse_draw_rrect() {
        let script: [u8; 16] = [
            0x00, 0x05, 0x00, 0x03, 0x42, 0x20, 0x00, 0x00, 0x41, 0xA0, 0x00, 0x00, 0x41, 0x20,
            0x00, 0x00,
        ];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![ScriptOp::DrawRRect {
                width: 40.0,
                height: 20.0,
                radius: 10.0,
                flag: 0x03
            }]
        );
    }

    #[test]
    fn parse_draw_rrectv() {
        let script: [u8; 28] = [
            0x00, 0x0C, 0x00, 0x03, 0x42, 0x20, 0x00, 0x00, 0x41, 0xA0, 0x00, 0x00, 0x41, 0x20,
            0x00, 0x00, 0x41, 0x00, 0x00, 0x00, 0x41, 0x80, 0x00, 0x00, 0x40, 0x80, 0x00, 0x00,
        ];
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![ScriptOp::DrawRRectV {
                width: 40.0,
                height: 20.0,
                ul_radius: 10.0,
                ur_radius: 8.0,
                lr_radius: 16.0,
                ll_radius: 4.0,
                flag: 0x03
            }]
        );
    }

    #[test]
    fn parse_stroke_cap_join_miter() {
        let script: [u8; 6] = [
            0x00, 0x80, 0x00, 0x01, 0x00, 0x81, // cap round, join next
        ];
        let script = [script.as_slice(), &[0x00, 0x02, 0x00, 0x82, 0x00, 0x05]].concat();
        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![
                ScriptOp::StrokeCap(skia_safe::PaintCap::Round),
                ScriptOp::StrokeJoin(skia_safe::PaintJoin::Miter),
                ScriptOp::StrokeMiterLimit(5.0)
            ]
        );
    }

    #[test]
    fn parse_path_ops() {
        let mut script: Vec<u8> = Vec::new();
        script.extend_from_slice(&[0x00, 0x20, 0x00, 0x00]);
        script.extend_from_slice(&[0x00, 0x26, 0x00, 0x00]);
        push_f32(&mut script, 1.0);
        push_f32(&mut script, 2.0);
        script.extend_from_slice(&[0x00, 0x27, 0x00, 0x00]);
        push_f32(&mut script, 3.0);
        push_f32(&mut script, 4.0);
        script.extend_from_slice(&[0x00, 0x28, 0x00, 0x00]);
        push_f32(&mut script, 5.0);
        push_f32(&mut script, 6.0);
        push_f32(&mut script, 7.0);
        push_f32(&mut script, 8.0);
        push_f32(&mut script, 9.0);
        script.extend_from_slice(&[0x00, 0x29, 0x00, 0x00]);
        push_f32(&mut script, 1.0);
        push_f32(&mut script, 2.0);
        push_f32(&mut script, 3.0);
        push_f32(&mut script, 4.0);
        push_f32(&mut script, 5.0);
        push_f32(&mut script, 6.0);
        script.extend_from_slice(&[0x00, 0x2A, 0x00, 0x00]);
        push_f32(&mut script, 7.0);
        push_f32(&mut script, 8.0);
        push_f32(&mut script, 9.0);
        push_f32(&mut script, 10.0);
        script.extend_from_slice(&[0x00, 0x21, 0x00, 0x00]);
        script.extend_from_slice(&[0x00, 0x22, 0x00, 0x00]);
        script.extend_from_slice(&[0x00, 0x23, 0x00, 0x00]);
        script.extend_from_slice(&[0x00, 0x44, 0x00, 0x00]);
        push_f32(&mut script, 30.0);
        push_f32(&mut script, 40.0);

        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![
                ScriptOp::BeginPath,
                ScriptOp::MoveTo { x: 1.0, y: 2.0 },
                ScriptOp::LineTo { x: 3.0, y: 4.0 },
                ScriptOp::ArcTo {
                    x1: 5.0,
                    y1: 6.0,
                    x2: 7.0,
                    y2: 8.0,
                    radius: 9.0
                },
                ScriptOp::BezierTo {
                    cp1x: 1.0,
                    cp1y: 2.0,
                    cp2x: 3.0,
                    cp2y: 4.0,
                    x: 5.0,
                    y: 6.0
                },
                ScriptOp::QuadraticTo {
                    cpx: 7.0,
                    cpy: 8.0,
                    x: 9.0,
                    y: 10.0
                },
                ScriptOp::ClosePath,
                ScriptOp::FillPath,
                ScriptOp::StrokePath,
                ScriptOp::Scissor {
                    width: 30.0,
                    height: 40.0
                }
            ]
        );
    }

    #[test]
    fn parse_path_shape_ops() {
        let mut script: Vec<u8> = Vec::new();
        script.extend_from_slice(&[0x00, 0x20, 0x00, 0x00]);
        script.extend_from_slice(&[0x00, 0x2B, 0x00, 0x00]);
        push_f32(&mut script, 1.0);
        push_f32(&mut script, 2.0);
        push_f32(&mut script, 3.0);
        push_f32(&mut script, 4.0);
        push_f32(&mut script, 5.0);
        push_f32(&mut script, 6.0);
        script.extend_from_slice(&[0x00, 0x2C, 0x00, 0x00]);
        push_f32(&mut script, 7.0);
        push_f32(&mut script, 8.0);
        push_f32(&mut script, 9.0);
        push_f32(&mut script, 10.0);
        push_f32(&mut script, 11.0);
        push_f32(&mut script, 12.0);
        push_f32(&mut script, 13.0);
        push_f32(&mut script, 14.0);
        script.extend_from_slice(&[0x00, 0x2D, 0x00, 0x00]);
        push_f32(&mut script, 15.0);
        push_f32(&mut script, 16.0);
        script.extend_from_slice(&[0x00, 0x2E, 0x00, 0x00]);
        push_f32(&mut script, 17.0);
        push_f32(&mut script, 18.0);
        push_f32(&mut script, 19.0);
        script.extend_from_slice(&[0x00, 0x2F, 0x00, 0x00]);
        push_f32(&mut script, 20.0);
        push_f32(&mut script, 1.5);
        script.extend_from_slice(&[0x00, 0x30, 0x00, 0x00]);
        push_f32(&mut script, 21.0);
        script.extend_from_slice(&[0x00, 0x31, 0x00, 0x00]);
        push_f32(&mut script, 22.0);
        push_f32(&mut script, 23.0);
        script.extend_from_slice(&[0x00, 0x32, 0x00, 0x00]);
        push_f32(&mut script, 24.0);
        push_f32(&mut script, 25.0);
        push_f32(&mut script, 26.0);
        push_f32(&mut script, 0.1);
        push_f32(&mut script, 0.2);
        script.extend_from_slice(&1u32.to_be_bytes());

        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![
                ScriptOp::BeginPath,
                ScriptOp::PathTriangle {
                    x0: 1.0,
                    y0: 2.0,
                    x1: 3.0,
                    y1: 4.0,
                    x2: 5.0,
                    y2: 6.0,
                },
                ScriptOp::PathQuad {
                    x0: 7.0,
                    y0: 8.0,
                    x1: 9.0,
                    y1: 10.0,
                    x2: 11.0,
                    y2: 12.0,
                    x3: 13.0,
                    y3: 14.0,
                },
                ScriptOp::PathRect {
                    width: 15.0,
                    height: 16.0
                },
                ScriptOp::PathRRect {
                    width: 17.0,
                    height: 18.0,
                    radius: 19.0
                },
                ScriptOp::PathSector {
                    radius: 20.0,
                    radians: 1.5
                },
                ScriptOp::PathCircle { radius: 21.0 },
                ScriptOp::PathEllipse {
                    radius0: 22.0,
                    radius1: 23.0
                },
                ScriptOp::PathArc {
                    cx: 24.0,
                    cy: 25.0,
                    radius: 26.0,
                    start: 0.1,
                    end: 0.2,
                    dir: 1
                }
            ]
        );
    }

    #[test]
    fn parse_linear_gradients() {
        let mut script: Vec<u8> = Vec::new();
        script.extend_from_slice(&[0x00, 0x61, 0x00, 0x00]);
        push_f32(&mut script, 1.0);
        push_f32(&mut script, 2.0);
        push_f32(&mut script, 3.0);
        push_f32(&mut script, 4.0);
        script.extend_from_slice(&[10, 20, 30, 40, 50, 60, 70, 80]);
        script.extend_from_slice(&[0x00, 0x72, 0x00, 0x00]);
        push_f32(&mut script, 5.0);
        push_f32(&mut script, 6.0);
        push_f32(&mut script, 7.0);
        push_f32(&mut script, 8.0);
        script.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

        let ops = parse_script(&script).expect("parse_script failed");
        assert_eq!(
            ops,
            vec![
                ScriptOp::FillLinear {
                    start_x: 1.0,
                    start_y: 2.0,
                    end_x: 3.0,
                    end_y: 4.0,
                    start_color: skia_safe::Color::from_argb(40, 10, 20, 30),
                    end_color: skia_safe::Color::from_argb(80, 50, 60, 70),
                },
                ScriptOp::StrokeLinear {
                    start_x: 5.0,
                    start_y: 6.0,
                    end_x: 7.0,
                    end_y: 8.0,
                    start_color: skia_safe::Color::from_argb(4, 1, 2, 3),
                    end_color: skia_safe::Color::from_argb(8, 5, 6, 7),
                }
            ]
        );
    }

    fn push_f32(buf: &mut Vec<u8>, value: f32) {
        buf.extend_from_slice(&value.to_bits().to_be_bytes());
    }
}
