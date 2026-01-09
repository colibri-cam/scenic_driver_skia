mod backend;
mod kms_backend;
mod raster_backend;
mod renderer;

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;
use std::time::Duration;

use backend::UserEvent;
use renderer::{RenderState, ScriptOp};
use rustler::types::{atom::Atom, list::ListIterator};
use rustler::wrapper::tuple as tuple_wrapper;

mod atoms {
    rustler::atoms! {
        push_state,
        pop_state,
        pop_push_state,
        translate,
        fill_color,
        draw_rect,
        draw_text,
        script,
        fill,
        fill_stroke,
        stroke,
        color_rgba,
    }
}

enum StopSignal {
    Wayland(winit::event_loop::EventLoopProxy<UserEvent>),
    Drm(Arc<AtomicBool>),
    Raster(Arc<AtomicBool>),
}

struct DriverHandle {
    stop: StopSignal,
    text: Arc<Mutex<String>>,
    render_state: Arc<Mutex<RenderState>>,
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
pub fn start(backend: Option<String>) -> Result<(), String> {
    let backend = backend
        .map(|b| b.to_lowercase())
        .map(|b| if b == "kms" { String::from("drm") } else { b })
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
    let running = Arc::new(AtomicBool::new(true));
    let handle = if backend == "drm" {
        let stop = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(false));
        let text_for_thread = Arc::clone(&text);
        let state_for_thread = Arc::clone(&render_state);
        let dirty_for_thread = Arc::clone(&dirty);
        let stop_for_thread = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                kms_backend::run(
                    stop_for_thread,
                    text_for_thread,
                    dirty_for_thread,
                    state_for_thread,
                )
            })
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        DriverHandle {
            stop: StopSignal::Drm(stop),
            text,
            render_state,
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
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                raster_backend::run(
                    stop_for_thread,
                    dirty_for_thread,
                    state_for_thread,
                    output_for_thread,
                    text_for_thread,
                )
            })
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        DriverHandle {
            stop: StopSignal::Raster(stop),
            text,
            render_state,
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
        let initial_state = render_state
            .lock()
            .map_err(|_| "driver state lock poisoned".to_string())?
            .clone();
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || backend::run(proxy_tx, initial_text, running_for_thread, initial_state))
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        let proxy = proxy_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "renderer did not initialize in time".to_string())?;
        DriverHandle {
            stop: StopSignal::Wayland(proxy),
            text,
            render_state,
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

#[rustler::nif(schedule = "DirtyCpu")]
pub fn submit_script_terms(script: rustler::Term) -> Result<(), String> {
    update_render_state(|state| {
        let ops = parse_script_terms(script)?;
        set_script(state, ROOT_ID.to_string(), ops);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn submit_script_terms2(script: rustler::Term) -> Result<(), String> {
    update_render_state(|state| {
        let ops = parse_script_terms2(script)?;
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
    let render_state_snapshot = render_state.clone();
    drop(render_state);

    match &handle.stop {
        StopSignal::Wayland(proxy) => proxy
            .send_event(UserEvent::SetRenderState(render_state_snapshot))
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
                rest = &tail[total..];
            }
            0x91 | 0x92 | 0x93 => {
                if rest.len() < 2 {
                    return Err("text style opcode truncated".to_string());
                }
                rest = &rest[2..];
            }
            _ => {
                return Err(format!("unsupported opcode: 0x{opcode:02x}"));
            }
        }
    }
    Ok(ops)
}

fn parse_script_terms(script: rustler::Term) -> Result<Vec<ScriptOp>, String> {
    let atom_push_state = atoms::push_state();
    let atom_pop_state = atoms::pop_state();
    let atom_pop_push_state = atoms::pop_push_state();
    let atom_translate = atoms::translate();
    let atom_fill_color = atoms::fill_color();
    let atom_draw_rect = atoms::draw_rect();
    let atom_draw_text = atoms::draw_text();
    let atom_script = atoms::script();
    let atom_fill = atoms::fill();
    let atom_fill_stroke = atoms::fill_stroke();
    let atom_stroke = atoms::stroke();
    let atom_color_rgba = atoms::color_rgba();

    let iter: ListIterator = script
        .decode()
        .map_err(|_| "script must be a list".to_string())?;
    let (lower, _) = iter.size_hint();
    let mut ops = Vec::with_capacity(lower);

    for op in iter {
        if op.is_atom() {
            if atom_push_state == op {
                ops.push(ScriptOp::PushState);
                continue;
            }
            if atom_pop_state == op {
                ops.push(ScriptOp::PopState);
                continue;
            }
            if atom_pop_push_state == op {
                ops.push(ScriptOp::PopPushState);
                continue;
            }
            return Err("unsupported atom op".to_string());
        }

        if !op.is_tuple() {
            return Err("unsupported op term".to_string());
        }

        let elements = tuple_terms(op).map_err(|_| "op tuple decode failed".to_string())?;
        if elements.len() != 2 {
            return Err("unsupported op tuple arity".to_string());
        }

        let env = op.get_env();
        let tag = unsafe { rustler::Term::new(env, elements[0]) };
        let payload = unsafe { rustler::Term::new(env, elements[1]) };

        if atom_translate == tag {
            let values =
                tuple_terms(payload).map_err(|_| "translate tuple decode failed".to_string())?;
            if values.len() != 2 {
                return Err("translate tuple arity mismatch".to_string());
            }
            let x = decode_f32(unsafe { rustler::Term::new(env, values[0]) })?;
            let y = decode_f32(unsafe { rustler::Term::new(env, values[1]) })?;
            ops.push(ScriptOp::Translate(x, y));
            continue;
        }

        if atom_fill_color == tag {
            let (r, g, b, a) = decode_color_rgba(payload, atom_color_rgba)?;
            ops.push(ScriptOp::FillColor(skia_safe::Color::from_argb(a, r, g, b)));
            continue;
        }

        if atom_draw_rect == tag {
            let values =
                tuple_terms(payload).map_err(|_| "draw_rect tuple decode failed".to_string())?;
            if values.len() != 3 {
                return Err("draw_rect tuple arity mismatch".to_string());
            }
            let width = decode_f32(unsafe { rustler::Term::new(env, values[0]) })?;
            let height = decode_f32(unsafe { rustler::Term::new(env, values[1]) })?;
            let flag = decode_flag(
                unsafe { rustler::Term::new(env, values[2]) },
                atom_fill,
                atom_fill_stroke,
                atom_stroke,
            )?;
            ops.push(ScriptOp::DrawRect {
                width,
                height,
                flag,
            });
            continue;
        }

        if atom_draw_text == tag {
            let text = payload
                .decode::<String>()
                .map_err(|_| "draw_text payload decode failed".to_string())?;
            ops.push(ScriptOp::DrawText(text));
            continue;
        }

        if atom_script == tag {
            let id = payload
                .decode::<String>()
                .map_err(|_| "draw_script id decode failed".to_string())?;
            ops.push(ScriptOp::DrawScript(id));
            continue;
        }

        return Err("unsupported op".to_string());
    }

    Ok(ops)
}

fn parse_script_terms2(script: rustler::Term) -> Result<Vec<ScriptOp>, String> {
    let atom_push_state = atoms::push_state();
    let atom_pop_state = atoms::pop_state();
    let atom_pop_push_state = atoms::pop_push_state();
    let atom_translate = atoms::translate();
    let atom_fill_color = atoms::fill_color();
    let atom_draw_rect = atoms::draw_rect();
    let atom_draw_text = atoms::draw_text();
    let atom_script = atoms::script();
    let atom_fill = atoms::fill();
    let atom_fill_stroke = atoms::fill_stroke();
    let atom_stroke = atoms::stroke();
    let atom_color_rgba = atoms::color_rgba();

    let iter: ListIterator = script
        .decode()
        .map_err(|_| "script must be a list".to_string())?;
    let (lower, _) = iter.size_hint();
    let mut ops = Vec::with_capacity(lower);
    for op in iter {
        if op.is_atom() {
            if atom_push_state == op {
                ops.push(ScriptOp::PushState);
                continue;
            }
            if atom_pop_state == op {
                ops.push(ScriptOp::PopState);
                continue;
            }
            if atom_pop_push_state == op {
                ops.push(ScriptOp::PopPushState);
                continue;
            }
            return Err("unsupported atom op".to_string());
        }

        if !op.is_tuple() {
            return Err("unsupported op term".to_string());
        }

        let elements = tuple_terms(op).map_err(|_| "op tuple decode failed".to_string())?;
        if elements.len() != 2 {
            return Err("unsupported op tuple arity".to_string());
        }

        let env = op.get_env();
        let tag = unsafe { rustler::Term::new(env, elements[0]) };
        let payload = unsafe { rustler::Term::new(env, elements[1]) };

        if atom_translate == tag {
            let values =
                tuple_terms(payload).map_err(|_| "translate tuple decode failed".to_string())?;
            if values.len() != 2 {
                return Err("translate tuple arity mismatch".to_string());
            }
            let x = decode_f32(unsafe { rustler::Term::new(env, values[0]) })?;
            let y = decode_f32(unsafe { rustler::Term::new(env, values[1]) })?;
            ops.push(ScriptOp::Translate(x, y));
            continue;
        }

        if atom_fill_color == tag {
            let (r, g, b, a) = decode_color_rgba(payload, atom_color_rgba)?;
            ops.push(ScriptOp::FillColor(skia_safe::Color::from_argb(a, r, g, b)));
            continue;
        }

        if atom_draw_rect == tag {
            let values =
                tuple_terms(payload).map_err(|_| "draw_rect tuple decode failed".to_string())?;
            if values.len() != 3 {
                return Err("draw_rect tuple arity mismatch".to_string());
            }
            let width = decode_f32(unsafe { rustler::Term::new(env, values[0]) })?;
            let height = decode_f32(unsafe { rustler::Term::new(env, values[1]) })?;
            let flag = decode_flag(
                unsafe { rustler::Term::new(env, values[2]) },
                atom_fill,
                atom_fill_stroke,
                atom_stroke,
            )?;
            ops.push(ScriptOp::DrawRect {
                width,
                height,
                flag,
            });
            continue;
        }

        if atom_draw_text == tag {
            let text = payload
                .decode::<String>()
                .map_err(|_| "draw_text payload decode failed".to_string())?;
            ops.push(ScriptOp::DrawText(text));
            continue;
        }

        if atom_script == tag {
            let id = payload
                .decode::<String>()
                .map_err(|_| "draw_script id decode failed".to_string())?;
            ops.push(ScriptOp::DrawScript(id));
            continue;
        }

        return Err("unsupported op".to_string());
    }

    Ok(ops)
}

fn decode_f32(term: rustler::Term) -> Result<f32, String> {
    if let Ok(value) = term.decode::<f64>() {
        return Ok(value as f32);
    }
    if let Ok(value) = term.decode::<i64>() {
        return Ok(value as f32);
    }
    Err("expected numeric value".to_string())
}

fn decode_u8(term: rustler::Term) -> Result<u8, String> {
    if let Ok(value) = term.decode::<u8>() {
        return Ok(value);
    }
    if let Ok(value) = term.decode::<u64>() {
        if value > u8::MAX as u64 {
            return Err("value out of u8 range".to_string());
        }
        return Ok(value as u8);
    }
    if let Ok(value) = term.decode::<i64>() {
        if value < 0 || value > u8::MAX as i64 {
            return Err("value out of u8 range".to_string());
        }
        return Ok(value as u8);
    }
    Err("expected u8 value".to_string())
}

fn decode_flag(
    term: rustler::Term,
    atom_fill: Atom,
    atom_fill_stroke: Atom,
    atom_stroke: Atom,
) -> Result<u16, String> {
    if term.is_atom() {
        if atom_fill == term {
            return Ok(0x01);
        }
        if atom_fill_stroke == term {
            return Ok(0x03);
        }
        if atom_stroke == term {
            return Ok(0x02);
        }
        return Err("unsupported draw flag atom".to_string());
    }

    if let Ok(value) = term.decode::<u16>() {
        return Ok(value);
    }
    if let Ok(value) = term.decode::<u64>() {
        return Ok(value as u16);
    }
    if let Ok(value) = term.decode::<i64>() {
        if value < 0 {
            return Err("draw flag out of range".to_string());
        }
        return Ok(value as u16);
    }
    Err("unsupported draw flag term".to_string())
}

fn decode_color_rgba(
    term: rustler::Term,
    atom_color_rgba: Atom,
) -> Result<(u8, u8, u8, u8), String> {
    let parts = tuple_terms(term).map_err(|_| "fill_color tuple decode failed".to_string())?;

    if parts.len() == 4 {
        return Ok((
            decode_u8(unsafe { rustler::Term::new(term.get_env(), parts[0]) })?,
            decode_u8(unsafe { rustler::Term::new(term.get_env(), parts[1]) })?,
            decode_u8(unsafe { rustler::Term::new(term.get_env(), parts[2]) })?,
            decode_u8(unsafe { rustler::Term::new(term.get_env(), parts[3]) })?,
        ));
    }

    if parts.len() != 2 {
        return Err("fill_color tuple arity mismatch".to_string());
    }

    let env = term.get_env();
    let tag = unsafe { rustler::Term::new(env, parts[0]) };
    if atom_color_rgba != tag {
        return Err("unsupported fill_color format".to_string());
    }

    let rgba = tuple_terms(unsafe { rustler::Term::new(env, parts[1]) })
        .map_err(|_| "color_rgba tuple decode failed".to_string())?;
    if rgba.len() != 4 {
        return Err("color_rgba tuple arity mismatch".to_string());
    }

    Ok((
        decode_u8(unsafe { rustler::Term::new(env, rgba[0]) })?,
        decode_u8(unsafe { rustler::Term::new(env, rgba[1]) })?,
        decode_u8(unsafe { rustler::Term::new(env, rgba[2]) })?,
        decode_u8(unsafe { rustler::Term::new(env, rgba[3]) })?,
    ))
}

fn tuple_terms<'a>(term: rustler::Term<'a>) -> Result<&'a [rustler::wrapper::NIF_TERM], String> {
    unsafe {
        tuple_wrapper::get_tuple(term.get_env().as_c_arg(), term.as_c_arg())
            .map_err(|_| "tuple decode failed".to_string())
    }
}

rustler::init!("Elixir.ScenicDriverSkia.Native");

#[cfg(test)]
mod tests {
    use super::*;

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
}
