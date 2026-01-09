use std::fs::{File, OpenOptions};
use std::os::fd::{AsFd, BorrowedFd};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use drm::Device as BasicDevice;
use drm::buffer::{Buffer, DrmFourcc};
use drm::control::{Device as ControlDevice, Mode, connector, crtc};
use skia_safe::{ColorType, ImageInfo, surfaces};

use crate::renderer::{RenderState, Renderer};

struct Card(File);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl BasicDevice for Card {}
impl ControlDevice for Card {}

fn open_card() -> Result<Card, String> {
    let card_path =
        std::env::var("SCENIC_DRM_CARD").unwrap_or_else(|_| String::from("/dev/dri/card0"));

    let fd = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&card_path)
        .map_err(|e| format!("failed to open {card_path}: {e}"))?;

    Ok(Card(fd))
}

fn first_connected_connector(
    card: &Card,
) -> Result<(connector::Handle, Mode, crtc::Handle), String> {
    let resources = card
        .resource_handles()
        .map_err(|e| format!("could not fetch DRM resources: {e}"))?;

    for handle in resources.connectors() {
        let info = card
            .get_connector(*handle, false)
            .map_err(|e| format!("failed to read connector {handle:?}: {e}"))?;

        if info.state() != connector::State::Connected {
            continue;
        }

        let mode = info
            .modes()
            .first()
            .cloned()
            .ok_or_else(|| format!("connector {handle:?} has no modes"))?;

        let crtc = resources
            .crtcs()
            .first()
            .copied()
            .ok_or_else(|| "no available CRTCs".to_string())?;

        return Ok((*handle, mode, crtc));
    }

    Err("no connected DRM connectors found".into())
}

pub fn run(
    stop: Arc<AtomicBool>,
    text: Arc<Mutex<String>>,
    dirty: Arc<AtomicBool>,
    render_state: Arc<Mutex<RenderState>>,
) {
    let card = match open_card() {
        Ok(card) => card,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let (connector, mode, crtc_handle) = match first_connected_connector(&card) {
        Ok(values) => values,
        Err(e) => {
            eprintln!("DRM backend unavailable: {e}");
            return;
        }
    };

    let (width, height) = mode.size();

    let mut dumb_buffer =
        match card.create_dumb_buffer((width.into(), height.into()), DrmFourcc::Xrgb8888, 32) {
            Ok(buffer) => buffer,
            Err(e) => {
                eprintln!("Could not create dumb buffer: {e}");
                return;
            }
        };

    let framebuffer = match card.add_framebuffer(&dumb_buffer, 24, 32) {
        Ok(fb) => fb,
        Err(e) => {
            eprintln!("Could not create framebuffer: {e}");
            return;
        }
    };

    if let Err(e) = card.set_crtc(
        crtc_handle,
        Some(framebuffer),
        (0, 0),
        &[connector],
        Some(mode),
    ) {
        eprintln!("Failed to set CRTC: {e}");
        return;
    }

    let stride = dumb_buffer.pitch() as usize;

    let mut mapping = match card.map_dumb_buffer(&mut dumb_buffer) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("Could not map dumb buffer: {e}");
            return;
        }
    };

    let image_info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::BGRA8888,
        skia_safe::AlphaType::Premul,
        None,
    );

    let surface = surfaces::wrap_pixels(&image_info, mapping.as_mut(), stride, None)
        .map(|borrowed| unsafe { borrowed.release() })
        .expect("Failed to create raster surface for KMS");

    let initial_text = text.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let mut renderer = Renderer::from_surface(surface, None, initial_text);
    if let Ok(state) = render_state.lock() {
        renderer.redraw(&state);
    }

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if dirty.swap(false, Ordering::Relaxed) {
            let updated = text.lock().unwrap_or_else(|e| e.into_inner()).clone();
            renderer.set_text(updated);
            if let Ok(state) = render_state.lock() {
                renderer.redraw(&state);
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
