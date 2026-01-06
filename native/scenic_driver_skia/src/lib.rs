mod backend;
mod kms_backend;
mod renderer;

use std::thread;

#[rustler::nif(schedule = "DirtyIo")]
pub fn start(backend: Option<String>) -> Result<(), String> {
    let backend = backend
        .map(|b| b.to_lowercase())
        .map(|b| if b == "kms" { String::from("drm") } else { b })
        .unwrap_or_else(|| String::from("wayland"));

    thread::Builder::new()
        .name(format!("scenic-driver-{backend}"))
        .spawn(move || match backend.as_str() {
            "drm" => kms_backend::run(),
            _ => backend::run(),
        })
        .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;

    Ok(())
}

rustler::init!("Elixir.ScenicDriverSkia.Native", [start]);
