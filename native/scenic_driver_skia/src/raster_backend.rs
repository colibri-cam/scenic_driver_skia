use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use skia_safe::{ColorType, EncodedImageFormat, ImageInfo, surfaces};

use crate::renderer::{RenderState, Renderer};

fn write_png(surface: &mut skia_safe::Surface, path: &str) {
    let image = surface.image_snapshot();
    match image.encode(None, EncodedImageFormat::PNG, None) {
        Some(data) => {
            if let Err(err) = std::fs::write(path, data.as_bytes()) {
                eprintln!("Failed to write raster output to {path}: {err}");
            }
        }
        None => {
            eprintln!("Failed to encode raster output to PNG");
        }
    }
}

pub fn run(
    stop: Arc<AtomicBool>,
    dirty: Arc<AtomicBool>,
    render_state: Arc<Mutex<RenderState>>,
    output_path: Arc<Mutex<Option<String>>>,
    text: Arc<Mutex<String>>,
) {
    let width = 800;
    let height = 600;

    let image_info = ImageInfo::new(
        (width, height),
        ColorType::BGRA8888,
        skia_safe::AlphaType::Premul,
        None,
    );

    let surface =
        surfaces::raster(&image_info, None, None).expect("Failed to create raster surface");

    let initial_state = {
        let state = render_state.lock().unwrap_or_else(|e| e.into_inner());
        state.clone()
    };
    let initial_text = text.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let mut renderer = Renderer::from_surface(surface, None, initial_text, initial_state);
    renderer.redraw();

    if let Some(path) = output_path.lock().ok().and_then(|p| p.clone()) {
        write_png(renderer.surface_mut(), &path);
    }

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if dirty.swap(false, Ordering::Relaxed) {
            let updated_text = text.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let state = render_state.lock().unwrap_or_else(|e| e.into_inner());
            renderer.set_text(updated_text);
            renderer.set_state(state.clone());
            renderer.redraw();
            if let Some(path) = output_path.lock().ok().and_then(|p| p.clone()) {
                write_png(renderer.surface_mut(), &path);
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
