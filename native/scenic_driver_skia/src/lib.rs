mod backend;
mod kms_backend;
mod renderer;

use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;
use std::time::Duration;

use backend::UserEvent;

enum StopSignal {
    Wayland(winit::event_loop::EventLoopProxy<UserEvent>),
    Drm(Arc<AtomicBool>),
}

struct DriverHandle {
    stop: StopSignal,
    text: Arc<Mutex<String>>,
    dirty: Option<Arc<AtomicBool>>,
    running: Arc<AtomicBool>,
    thread: thread::JoinHandle<()>,
}

static DRIVER: OnceLock<Mutex<Option<DriverHandle>>> = OnceLock::new();

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
            StopSignal::Drm(_) => {
                return Err("renderer already running".to_string());
            }
        }
    }

    let thread_name = format!("scenic-driver-{backend}");
    let text = Arc::new(Mutex::new(String::from("Hello, Wayland")));
    let running = Arc::new(AtomicBool::new(true));
    let handle = if backend == "drm" {
        let stop = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(false));
        let text_for_thread = Arc::clone(&text);
        let dirty_for_thread = Arc::clone(&dirty);
        let stop_for_thread = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || kms_backend::run(stop_for_thread, text_for_thread, dirty_for_thread))
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        DriverHandle {
            stop: StopSignal::Drm(stop),
            text,
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
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || backend::run(proxy_tx, initial_text, running_for_thread))
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        let proxy = proxy_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "renderer did not initialize in time".to_string())?;
        DriverHandle {
            stop: StopSignal::Wayland(proxy),
            text,
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
    };
    handle.running.store(false, Ordering::Relaxed);

    match &handle.stop {
        StopSignal::Wayland(_) => signal_result,
        StopSignal::Drm(_) => {
            let handle = state.take().expect("handle checked");
            handle
                .thread
                .join()
                .map_err(|_| "renderer thread panicked".to_string())?;
            signal_result
        }
    }
}

#[rustler::nif]
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
        StopSignal::Drm(_) => {
            if let Some(dirty) = &handle.dirty {
                dirty.store(true, Ordering::Relaxed);
            }
            Ok(())
        }
    }
}

rustler::init!("Elixir.ScenicDriverSkia.Native");
