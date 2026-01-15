use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::Duration;

use skia_safe::{AlphaType, ColorType, ImageInfo, image::CachingHint, surfaces};

use crate::{
    RasterFrame,
    renderer::{RenderState, Renderer},
};

fn store_frame(
    renderer: &mut Renderer,
    frame_slot: &Arc<Mutex<Option<RasterFrame>>>,
    size: (u32, u32),
) {
    let (width, height) = size;
    let image = renderer.surface_mut().image_snapshot();
    let image_info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::RGB888x,
        AlphaType::Opaque,
        None,
    );
    let row_bytes = image_info.min_row_bytes();
    let mut pixels = vec![0u8; row_bytes * height as usize];
    let ok = image.read_pixels(
        &image_info,
        pixels.as_mut_slice(),
        row_bytes,
        (0, 0),
        CachingHint::Disallow,
    );
    if !ok {
        return;
    }

    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for chunk in pixels.chunks_exact(4) {
        rgb.push(chunk[0]);
        rgb.push(chunk[1]);
        rgb.push(chunk[2]);
    }

    if let Ok(mut slot) = frame_slot.lock() {
        *slot = Some(RasterFrame {
            width,
            height,
            data: rgb,
        });
    }
}

pub fn run(
    stop: Arc<AtomicBool>,
    dirty: Arc<AtomicBool>,
    render_state: Arc<Mutex<RenderState>>,
    frame_slot: Arc<Mutex<Option<RasterFrame>>>,
    input_mask: Arc<AtomicU32>,
    requested_size: Option<(u32, u32)>,
) {
    let _input_mask = input_mask;
    let (width, height) = requested_size.unwrap_or((800, 600));
    let width = width.max(1);
    let height = height.max(1);

    let image_info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None,
    );

    let surface =
        surfaces::raster(&image_info, None, None).expect("Failed to create raster surface");

    let mut renderer = Renderer::from_surface(surface, None);
    if let Ok(state) = render_state.lock() {
        renderer.redraw(&state);
    }

    store_frame(&mut renderer, &frame_slot, (width, height));

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if dirty.swap(false, Ordering::Relaxed) {
            if let Ok(state) = render_state.lock() {
                renderer.redraw(&state);
            }
            store_frame(&mut renderer, &frame_slot, (width, height));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
