mod backend;
mod drm_backend;
mod drm_input;
mod input;
mod input_translate;
mod raster_backend;
mod renderer;

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc,
};
use std::thread;
use std::time::Duration;

use backend::UserEvent;
use input::{InputEvent, InputQueue, notify_input_ready};
use renderer::{RenderState, ScriptOp};

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
    raster_output: Option<Arc<Mutex<Option<String>>>>,
    dirty: Option<Arc<AtomicBool>>,
    running: Arc<AtomicBool>,
    thread: thread::JoinHandle<()>,
}

static DRIVER: OnceLock<Mutex<Option<DriverHandle>>> = OnceLock::new();
const ROOT_ID: &str = "_root_";

fn driver_state() -> &'static Mutex<Option<DriverHandle>> {
    DRIVER.get_or_init(|| Mutex::new(None))
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn start(
    backend: Option<String>,
    viewport_size: Option<(u32, u32)>,
    window_title: String,
    window_resizeable: bool,
) -> Result<(), String> {
    let backend = backend
        .map(|b| b.to_lowercase())
        .unwrap_or_else(|| String::from("wayland"));

    let mut state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;

    if let Some(handle) = state.as_ref() {
        match &handle.stop {
            StopSignal::Wayland(proxy) => {
                if handle.running.load(Ordering::Relaxed) {
                    return Err("renderer already running".to_string());
                }
                handle.running.store(true, Ordering::Relaxed);
                let result = proxy
                    .send_event(UserEvent::Start)
                    .map_err(|err| format!("failed to signal renderer: {err}"));
                if result.is_err() {
                    handle.running.store(false, Ordering::Relaxed);
                }
                return result;
            }
            StopSignal::Drm(_) | StopSignal::Raster(_) => {
                return Err("renderer already running".to_string());
            }
        }
    }

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
                    requested_size,
                )
            })
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        DriverHandle {
            stop: StopSignal::Drm(stop),
            text,
            render_state,
            input_events,
            input_mask,
            raster_output: None,
            dirty: Some(dirty),
            running,
            thread,
        }
    } else if backend == "raster" {
        let stop = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(false));
        let state_for_thread = Arc::clone(&render_state);
        let dirty_for_thread = Arc::clone(&dirty);
        let stop_for_thread = Arc::clone(&stop);
        let text_for_thread = Arc::clone(&text);
        let raster_output = Arc::new(Mutex::new(None));
        let output_for_thread = Arc::clone(&raster_output);
        let input_for_thread = Arc::clone(&input_mask);
        let requested_size = viewport_size;
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                raster_backend::run(
                    stop_for_thread,
                    dirty_for_thread,
                    state_for_thread,
                    output_for_thread,
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
            raster_output: Some(raster_output),
            dirty: Some(dirty),
            running,
            thread,
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
            raster_output: None,
            dirty: None,
            running,
            thread,
        }
    };

    *state = Some(handle);

    Ok(())
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn stop() -> Result<(), String> {
    let mut state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;

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

    match &handle.stop {
        StopSignal::Wayland(_) => signal_result,
        StopSignal::Drm(_) | StopSignal::Raster(_) => {
            let handle = state.take().expect("handle checked");
            handle
                .thread
                .join()
                .map_err(|_| "renderer thread panicked".to_string())?;
            signal_result
        }
    }
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_text(text: String) -> Result<(), String> {
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;

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
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn reset_scene() -> Result<(), String> {
    update_render_state(|state| {
        state.scripts = HashMap::new();
        state.root_id = None;
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_clear_color(color: (u8, u8, u8, u8)) -> Result<(), String> {
    update_render_state(|state| {
        state.clear_color = skia_safe::Color::from_argb(color.3, color.0, color.1, color.2);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn submit_script(script: rustler::Binary) -> Result<(), String> {
    update_render_state(|state| {
        let ops = parse_script(script.as_slice())?;
        set_script(state, ROOT_ID.to_string(), ops);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn submit_script_with_id(id: String, script: rustler::Binary) -> Result<(), String> {
    update_render_state(|state| {
        let ops = parse_script(script.as_slice())?;
        set_script(state, id.clone(), ops);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn submit_scripts(scripts: Vec<(String, rustler::Binary)>) -> Result<(), String> {
    update_render_state(|state| {
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
pub fn del_script(id: String) -> Result<(), String> {
    update_render_state(|state| {
        state.scripts.remove(&id);
        if state.root_id.as_deref() == Some(id.as_str()) {
            state.root_id = None;
        }
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn script_count() -> Result<u64, String> {
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;
    let render_state = handle
        .render_state
        .lock()
        .map_err(|_| "render state lock poisoned".to_string())?;
    Ok(render_state.scripts.len() as u64)
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_raster_output(path: String) -> Result<(), String> {
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;
    let output = handle
        .raster_output
        .as_ref()
        .ok_or_else(|| "raster backend not active".to_string())?;

    let mut slot = output
        .lock()
        .map_err(|_| "raster output lock poisoned".to_string())?;
    *slot = Some(path);

    if let Some(dirty) = &handle.dirty {
        dirty.store(true, Ordering::Relaxed);
    }

    Ok(())
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_input_mask(mask: u32) -> Result<(), String> {
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;

    handle.input_mask.store(mask, Ordering::Relaxed);

    Ok(())
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_input_target(pid: Option<rustler::LocalPid>) -> Result<(), String> {
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;
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
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn drain_input_events() -> Result<Vec<InputEvent>, String> {
    drain_input_events_inner()
}

fn drain_input_events_inner() -> Result<Vec<InputEvent>, String> {
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;
    let mut queue = handle
        .input_events
        .lock()
        .map_err(|_| "input queue lock poisoned".to_string())?;
    Ok(queue.drain())
}

fn update_render_state<F>(mut update: F) -> Result<(), String>
where
    F: FnMut(&mut RenderState) -> Result<(), String>,
{
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;

    let mut render_state = handle
        .render_state
        .lock()
        .map_err(|_| "render state lock poisoned".to_string())?;
    update(&mut render_state)?;
    drop(render_state);

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

rustler::init!("Elixir.Scenic.Driver.Skia.Native");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputEvent, InputQueue};

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
    fn drain_input_events_returns_queued_events() {
        let mut state = driver_state().lock().expect("driver state lock");
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

        *state = Some(DriverHandle {
            stop: StopSignal::Raster(Arc::clone(&stop)),
            text: Arc::new(Mutex::new(String::new())),
            render_state: Arc::new(Mutex::new(RenderState::default())),
            input_events: Arc::clone(&input_events),
            input_mask: Arc::new(AtomicU32::new(0)),
            raster_output: None,
            dirty: Some(Arc::new(AtomicBool::new(false))),
            running: Arc::new(AtomicBool::new(false)),
            thread,
        });

        drop(state);
        let drained = drain_input_events_inner().expect("drain_input_events failed");
        assert_eq!(drained.len(), 3);
        assert!(matches!(drained[0], InputEvent::CursorPos { .. }));
        assert!(matches!(drained[1], InputEvent::Key { .. }));
        assert!(matches!(drained[2], InputEvent::ViewportReshape { .. }));

        let mut state = driver_state().lock().expect("driver state lock");
        if let Some(handle) = state.take() {
            let _ = handle.thread.join();
        }
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
}
