use std::fs;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, Ordering},
};

use evdev::{
    AbsoluteAxisType, Device, InputEventKind, Key, PropType, RelativeAxisType, Synchronization,
};
use libc::input_absinfo;

use crate::cursor::CursorState;
use crate::input::{
    ACTION_PRESS, ACTION_RELEASE, INPUT_MASK_CODEPOINT, INPUT_MASK_CURSOR_BUTTON,
    INPUT_MASK_CURSOR_POS, INPUT_MASK_CURSOR_SCROLL, INPUT_MASK_KEY, InputEvent, InputQueue,
    notify_input_ready,
};
use crate::input_translate::{
    Key as ScenicKey, KeyLocation, Modifiers, MouseButton, NamedKey, button_to_scenic,
    key_to_scenic, modifiers_to_mask,
};

struct InputDevice {
    device: Device,
    abs_x: Option<AbsAxisState>,
    abs_y: Option<AbsAxisState>,
    abs_x_dirty: bool,
    abs_y_dirty: bool,
    abs_mode: AbsMode,
    last_abs_scaled: Option<(f32, f32)>,
    touch_active: bool,
    touch_tracking: bool,
}

#[derive(Clone, Copy, Debug)]
struct AbsAxisState {
    value: i32,
    min: i32,
    max: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AbsMode {
    Absolute,
    RelativeFromAbs,
}

pub struct DrmInput {
    devices: Vec<InputDevice>,
    cursor_pos: (f32, f32),
    modifiers: Modifiers,
    caps_lock: bool,
    screen_size: (u32, u32),
    input_mask: Arc<AtomicU32>,
    input_events: Arc<Mutex<InputQueue>>,
    cursor_state: Arc<Mutex<CursorState>>,
}

impl DrmInput {
    pub fn new(
        screen_size: (u32, u32),
        input_mask: Arc<AtomicU32>,
        input_events: Arc<Mutex<InputQueue>>,
        cursor_state: Arc<Mutex<CursorState>>,
        log_enabled: bool,
    ) -> Self {
        let devices = enumerate_devices(log_enabled);
        Self {
            devices,
            cursor_pos: (0.0, 0.0),
            modifiers: Modifiers::default(),
            caps_lock: false,
            screen_size,
            input_mask,
            input_events,
            cursor_state,
        }
    }

    pub fn poll(&mut self) {
        let mask = self.input_mask.load(Ordering::Relaxed);
        if mask == 0 {
            return;
        }

        for idx in 0..self.devices.len() {
            let events = {
                let device = &mut self.devices[idx];
                match device.device.fetch_events() {
                    Ok(events) => events.collect::<Vec<_>>(),
                    Err(_) => Vec::new(),
                }
            };

            for event in events {
                match event.kind() {
                    InputEventKind::Key(key) => {
                        self.handle_key_event_with_device(idx, key, event.value(), mask);
                    }
                    InputEventKind::RelAxis(axis) => {
                        self.handle_rel_event(axis, event.value(), mask);
                    }
                    InputEventKind::AbsAxis(axis) => {
                        let device = &mut self.devices[idx];
                        update_abs_state(device, axis, event.value(), self.screen_size);
                    }
                    InputEventKind::Synchronization(sync) => {
                        if sync == Synchronization::SYN_REPORT {
                            let action = {
                                let device = &mut self.devices[idx];
                                consume_abs_action(device, self.screen_size)
                            };
                            match action {
                                AbsAction::Absolute(x, y) => self.handle_abs_position(x, y, mask),
                                AbsAction::Relative(dx, dy) => {
                                    self.handle_abs_relative(dx, dy, mask);
                                }
                                AbsAction::None => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_key_event(&mut self, key: Key, value: i32, mask: u32) {
        let pressed = value != 0;
        self.update_modifiers(key, pressed);
        if key == Key::KEY_CAPSLOCK && pressed {
            self.caps_lock = !self.caps_lock;
        }

        if let Some(button) = evdev_key_to_button(key) {
            if mask & INPUT_MASK_CURSOR_BUTTON != 0 {
                let (x, y) = self.cursor_pos;
                let action = if pressed {
                    ACTION_PRESS
                } else {
                    ACTION_RELEASE
                };
                let mods = modifiers_to_mask(self.modifiers);
                self.push_input(InputEvent::CursorButton {
                    button: button_to_scenic(button),
                    action,
                    mods,
                    x,
                    y,
                });
            }
            return;
        }

        let Some((key, location)) = evdev_key_to_scenic(key) else {
            return;
        };
        let mods = modifiers_to_mask(self.modifiers);
        let action = if pressed {
            ACTION_PRESS
        } else {
            ACTION_RELEASE
        };
        if mask & INPUT_MASK_KEY != 0 {
            self.push_input(InputEvent::Key {
                key: key_to_scenic(key, location),
                action,
                mods,
            });
        }

        if pressed
            && mask & INPUT_MASK_CODEPOINT != 0
            && let Some(codepoint) = key_to_codepoint(key, self.modifiers, self.caps_lock)
        {
            self.push_input(InputEvent::Codepoint { codepoint, mods });
        }
    }

    fn handle_rel_event(&mut self, axis: RelativeAxisType, value: i32, mask: u32) {
        let (mut x, mut y) = self.cursor_pos;
        match axis {
            RelativeAxisType::REL_X => {
                x += value as f32;
            }
            RelativeAxisType::REL_Y => {
                y += value as f32;
            }
            RelativeAxisType::REL_WHEEL => {
                if mask & INPUT_MASK_CURSOR_SCROLL != 0 {
                    let (cx, cy) = self.cursor_pos;
                    self.push_input(InputEvent::CursorScroll {
                        dx: 0.0,
                        dy: value as f32,
                        x: cx,
                        y: cy,
                    });
                }
                return;
            }
            RelativeAxisType::REL_HWHEEL => {
                if mask & INPUT_MASK_CURSOR_SCROLL != 0 {
                    let (cx, cy) = self.cursor_pos;
                    self.push_input(InputEvent::CursorScroll {
                        dx: value as f32,
                        dy: 0.0,
                        x: cx,
                        y: cy,
                    });
                }
                return;
            }
            _ => return,
        }

        let (width, height) = self.screen_size;
        x = x.clamp(0.0, width.saturating_sub(1) as f32);
        y = y.clamp(0.0, height.saturating_sub(1) as f32);
        self.set_cursor_pos(x, y);

        if mask & INPUT_MASK_CURSOR_POS != 0 {
            self.push_input(InputEvent::CursorPos { x, y });
        }
    }

    fn handle_abs_position(&mut self, x: f32, y: f32, mask: u32) {
        self.set_cursor_pos(x, y);
        if mask & INPUT_MASK_CURSOR_POS != 0 {
            self.push_input(InputEvent::CursorPos { x, y });
        }
    }

    fn handle_abs_relative(&mut self, dx: f32, dy: f32, mask: u32) {
        let (mut x, mut y) = self.cursor_pos;
        x += dx;
        y += dy;
        let (width, height) = self.screen_size;
        x = x.clamp(0.0, width.saturating_sub(1) as f32);
        y = y.clamp(0.0, height.saturating_sub(1) as f32);
        self.set_cursor_pos(x, y);
        if mask & INPUT_MASK_CURSOR_POS != 0 {
            self.push_input(InputEvent::CursorPos { x, y });
        }
    }

    fn set_cursor_pos(&mut self, x: f32, y: f32) {
        self.cursor_pos = (x, y);
        if let Ok(mut cursor) = self.cursor_state.lock() {
            cursor.pos = (x, y);
        }
    }

    fn update_modifiers(&mut self, key: Key, pressed: bool) {
        match key {
            Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT => self.modifiers.shift = pressed,
            Key::KEY_LEFTCTRL | Key::KEY_RIGHTCTRL => self.modifiers.ctrl = pressed,
            Key::KEY_LEFTALT | Key::KEY_RIGHTALT => self.modifiers.alt = pressed,
            Key::KEY_LEFTMETA | Key::KEY_RIGHTMETA => self.modifiers.meta = pressed,
            _ => {}
        }
    }

    fn push_input(&self, event: InputEvent) {
        let notify = if let Ok(mut queue) = self.input_events.lock() {
            queue.push_event(event)
        } else {
            None
        };

        if let Some(pid) = notify {
            notify_input_ready(pid);
        }
    }
}

fn enumerate_devices(log_enabled: bool) -> Vec<InputDevice> {
    let mut devices = Vec::new();
    let entries = match fs::read_dir("/dev/input") {
        Ok(entries) => entries,
        Err(_) => return devices,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_event_device(&path) {
            continue;
        }
        let device = match Device::open(&path) {
            Ok(device) => device,
            Err(_) => continue,
        };
        set_non_blocking(device.as_raw_fd());
        let (abs_mode, info) = detect_abs_mode(&device);
        let (abs_x, abs_y) = init_abs_axes(&device);
        if log_enabled {
            let name = device.name().unwrap_or("unknown");
            eprintln!(
                "drm_input device={:?} name=\"{}\" abs_mode={:?} {}",
                path, name, abs_mode, info
            );
        }
        devices.push(InputDevice {
            device,
            abs_x,
            abs_y,
            abs_x_dirty: false,
            abs_y_dirty: false,
            abs_mode,
            last_abs_scaled: None,
            touch_active: false,
            touch_tracking: false,
        });
    }

    devices
}

fn is_event_device(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with("event"))
        .unwrap_or(false)
}

enum AbsAction {
    None,
    Absolute(f32, f32),
    Relative(f32, f32),
}

fn update_abs_state(
    device: &mut InputDevice,
    axis: AbsoluteAxisType,
    value: i32,
    screen_size: (u32, u32),
) {
    let fallback = (
        screen_size.0.saturating_sub(1) as i32,
        screen_size.1.saturating_sub(1) as i32,
    );
    match axis {
        AbsoluteAxisType::ABS_X => {
            device.abs_x = Some(update_axis_state(device.abs_x, value, fallback.0));
            device.abs_x_dirty = true;
        }
        AbsoluteAxisType::ABS_Y => {
            device.abs_y = Some(update_axis_state(device.abs_y, value, fallback.1));
            device.abs_y_dirty = true;
        }
        _ => {}
    }
}

fn consume_abs_action(device: &mut InputDevice, screen_size: (u32, u32)) -> AbsAction {
    if !(device.abs_x_dirty && device.abs_y_dirty) {
        return AbsAction::None;
    }

    let (abs_x, abs_y) = match (device.abs_x, device.abs_y) {
        (Some(abs_x), Some(abs_y)) => (abs_x, abs_y),
        _ => return AbsAction::None,
    };

    let scaled = (
        scale_abs_value(abs_x, screen_size.0),
        scale_abs_value(abs_y, screen_size.1),
    );
    device.abs_x_dirty = false;
    device.abs_y_dirty = false;

    if device.abs_mode == AbsMode::RelativeFromAbs {
        if device.touch_tracking && !device.touch_active {
            device.last_abs_scaled = Some(scaled);
            return AbsAction::None;
        }
        let (dx, dy) = match device.last_abs_scaled {
            Some((last_x, last_y)) => (scaled.0 - last_x, scaled.1 - last_y),
            None => (0.0, 0.0),
        };
        device.last_abs_scaled = Some(scaled);
        AbsAction::Relative(dx, dy)
    } else {
        AbsAction::Absolute(scaled.0, scaled.1)
    }
}
fn update_axis_state(current: Option<AbsAxisState>, value: i32, fallback_max: i32) -> AbsAxisState {
    match current {
        Some(mut state) => {
            state.value = value;
            state
        }
        None => AbsAxisState {
            value,
            min: 0,
            max: fallback_max,
        },
    }
}

fn scale_abs_value(state: AbsAxisState, screen_max: u32) -> f32 {
    let screen_max = screen_max.saturating_sub(1) as f32;
    if screen_max <= 0.0 {
        return 0.0;
    }
    let min = state.min as f32;
    let max = state.max as f32;
    if max <= min {
        return (state.value as f32).clamp(0.0, screen_max);
    }
    let norm = ((state.value as f32 - min) / (max - min)).clamp(0.0, 1.0);
    norm * screen_max
}

fn init_abs_axes(device: &Device) -> (Option<AbsAxisState>, Option<AbsAxisState>) {
    let Ok(abs_state) = device.get_abs_state() else {
        return (None, None);
    };

    let abs_x = axis_state_from_abs(abs_state.get(AbsoluteAxisType::ABS_X.0 as usize));
    let abs_y = axis_state_from_abs(abs_state.get(AbsoluteAxisType::ABS_Y.0 as usize));
    (abs_x, abs_y)
}

fn axis_state_from_abs(info: Option<&input_absinfo>) -> Option<AbsAxisState> {
    info.map(|info| AbsAxisState {
        value: info.value,
        min: info.minimum,
        max: info.maximum,
    })
}

fn detect_abs_mode(device: &Device) -> (AbsMode, String) {
    let has_abs = device.supported_absolute_axes().is_some_and(|axes| {
        axes.contains(AbsoluteAxisType::ABS_X) && axes.contains(AbsoluteAxisType::ABS_Y)
    });
    if !has_abs {
        return (AbsMode::Absolute, "abs_axes=none".to_string());
    }

    let props = device.properties();
    let direct_prop = props.contains(PropType::DIRECT);
    let buttonpad_prop = props.contains(PropType::BUTTONPAD);
    let topbuttonpad_prop = props.contains(PropType::TOPBUTTONPAD);
    let semi_mt_prop = props.contains(PropType::SEMI_MT);
    let pointer_prop = props.contains(PropType::POINTER);

    let key_hint = device.supported_keys().is_some_and(|keys| {
        keys.contains(Key::BTN_TOOL_FINGER)
            || keys.contains(Key::BTN_TOUCH)
            || keys.contains(Key::BTN_TOOL_DOUBLETAP)
            || keys.contains(Key::BTN_TOOL_TRIPLETAP)
            || keys.contains(Key::BTN_TOOL_QUADTAP)
            || keys.contains(Key::BTN_TOOL_QUINTTAP)
    });

    let name_hint = device
        .name()
        .map(|name| name.to_ascii_lowercase().contains("touchpad"))
        .unwrap_or(false);

    let touchpad = !direct_prop
        && ((pointer_prop && (buttonpad_prop || topbuttonpad_prop || semi_mt_prop || key_hint))
            || name_hint);

    let info = format!(
        "abs_axes=xy direct={} pointer={} buttonpad={} topbuttonpad={} semi_mt={} key_hint={} name_hint={}",
        direct_prop,
        pointer_prop,
        buttonpad_prop,
        topbuttonpad_prop,
        semi_mt_prop,
        key_hint,
        name_hint
    );

    if touchpad {
        (AbsMode::RelativeFromAbs, info)
    } else {
        (AbsMode::Absolute, info)
    }
}

impl DrmInput {
    fn handle_key_event_with_device(&mut self, idx: usize, key: Key, value: i32, mask: u32) {
        let pressed = value != 0;
        if let Some(device) = self.devices.get_mut(idx)
            && device.abs_mode == AbsMode::RelativeFromAbs
            && is_touch_tracking_key(key)
        {
            device.touch_tracking = true;
            device.touch_active = pressed;
            if pressed {
                device.last_abs_scaled = None;
            }
        }

        self.handle_key_event(key, value, mask);
    }
}

fn is_touch_tracking_key(key: Key) -> bool {
    matches!(
        key,
        Key::BTN_TOUCH
            | Key::BTN_TOOL_FINGER
            | Key::BTN_TOOL_DOUBLETAP
            | Key::BTN_TOOL_TRIPLETAP
            | Key::BTN_TOOL_QUADTAP
            | Key::BTN_TOOL_QUINTTAP
    )
}

fn set_non_blocking(fd: i32) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags >= 0 {
            let _ = libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }
}

fn evdev_key_to_scenic(key: Key) -> Option<(ScenicKey, KeyLocation)> {
    let (key, location) = match key {
        Key::KEY_A => (ScenicKey::Character('a'), KeyLocation::Standard),
        Key::KEY_B => (ScenicKey::Character('b'), KeyLocation::Standard),
        Key::KEY_C => (ScenicKey::Character('c'), KeyLocation::Standard),
        Key::KEY_D => (ScenicKey::Character('d'), KeyLocation::Standard),
        Key::KEY_E => (ScenicKey::Character('e'), KeyLocation::Standard),
        Key::KEY_F => (ScenicKey::Character('f'), KeyLocation::Standard),
        Key::KEY_G => (ScenicKey::Character('g'), KeyLocation::Standard),
        Key::KEY_H => (ScenicKey::Character('h'), KeyLocation::Standard),
        Key::KEY_I => (ScenicKey::Character('i'), KeyLocation::Standard),
        Key::KEY_J => (ScenicKey::Character('j'), KeyLocation::Standard),
        Key::KEY_K => (ScenicKey::Character('k'), KeyLocation::Standard),
        Key::KEY_L => (ScenicKey::Character('l'), KeyLocation::Standard),
        Key::KEY_M => (ScenicKey::Character('m'), KeyLocation::Standard),
        Key::KEY_N => (ScenicKey::Character('n'), KeyLocation::Standard),
        Key::KEY_O => (ScenicKey::Character('o'), KeyLocation::Standard),
        Key::KEY_P => (ScenicKey::Character('p'), KeyLocation::Standard),
        Key::KEY_Q => (ScenicKey::Character('q'), KeyLocation::Standard),
        Key::KEY_R => (ScenicKey::Character('r'), KeyLocation::Standard),
        Key::KEY_S => (ScenicKey::Character('s'), KeyLocation::Standard),
        Key::KEY_T => (ScenicKey::Character('t'), KeyLocation::Standard),
        Key::KEY_U => (ScenicKey::Character('u'), KeyLocation::Standard),
        Key::KEY_V => (ScenicKey::Character('v'), KeyLocation::Standard),
        Key::KEY_W => (ScenicKey::Character('w'), KeyLocation::Standard),
        Key::KEY_X => (ScenicKey::Character('x'), KeyLocation::Standard),
        Key::KEY_Y => (ScenicKey::Character('y'), KeyLocation::Standard),
        Key::KEY_Z => (ScenicKey::Character('z'), KeyLocation::Standard),
        Key::KEY_0 => (ScenicKey::Character('0'), KeyLocation::Standard),
        Key::KEY_1 => (ScenicKey::Character('1'), KeyLocation::Standard),
        Key::KEY_2 => (ScenicKey::Character('2'), KeyLocation::Standard),
        Key::KEY_3 => (ScenicKey::Character('3'), KeyLocation::Standard),
        Key::KEY_4 => (ScenicKey::Character('4'), KeyLocation::Standard),
        Key::KEY_5 => (ScenicKey::Character('5'), KeyLocation::Standard),
        Key::KEY_6 => (ScenicKey::Character('6'), KeyLocation::Standard),
        Key::KEY_7 => (ScenicKey::Character('7'), KeyLocation::Standard),
        Key::KEY_8 => (ScenicKey::Character('8'), KeyLocation::Standard),
        Key::KEY_9 => (ScenicKey::Character('9'), KeyLocation::Standard),
        Key::KEY_SPACE => (ScenicKey::Character(' '), KeyLocation::Standard),
        Key::KEY_ENTER => (ScenicKey::Named(NamedKey::Enter), KeyLocation::Standard),
        Key::KEY_TAB => (ScenicKey::Named(NamedKey::Tab), KeyLocation::Standard),
        Key::KEY_ESC => (ScenicKey::Named(NamedKey::Escape), KeyLocation::Standard),
        Key::KEY_BACKSPACE => (ScenicKey::Named(NamedKey::Backspace), KeyLocation::Standard),
        Key::KEY_INSERT => (ScenicKey::Named(NamedKey::Insert), KeyLocation::Standard),
        Key::KEY_DELETE => (ScenicKey::Named(NamedKey::Delete), KeyLocation::Standard),
        Key::KEY_LEFT => (ScenicKey::Named(NamedKey::ArrowLeft), KeyLocation::Standard),
        Key::KEY_RIGHT => (
            ScenicKey::Named(NamedKey::ArrowRight),
            KeyLocation::Standard,
        ),
        Key::KEY_UP => (ScenicKey::Named(NamedKey::ArrowUp), KeyLocation::Standard),
        Key::KEY_DOWN => (ScenicKey::Named(NamedKey::ArrowDown), KeyLocation::Standard),
        Key::KEY_PAGEUP => (ScenicKey::Named(NamedKey::PageUp), KeyLocation::Standard),
        Key::KEY_PAGEDOWN => (ScenicKey::Named(NamedKey::PageDown), KeyLocation::Standard),
        Key::KEY_HOME => (ScenicKey::Named(NamedKey::Home), KeyLocation::Standard),
        Key::KEY_END => (ScenicKey::Named(NamedKey::End), KeyLocation::Standard),
        Key::KEY_CAPSLOCK => (ScenicKey::Named(NamedKey::CapsLock), KeyLocation::Standard),
        Key::KEY_SCROLLLOCK => (
            ScenicKey::Named(NamedKey::ScrollLock),
            KeyLocation::Standard,
        ),
        Key::KEY_NUMLOCK => (ScenicKey::Named(NamedKey::NumLock), KeyLocation::Standard),
        Key::KEY_SYSRQ => (
            ScenicKey::Named(NamedKey::PrintScreen),
            KeyLocation::Standard,
        ),
        Key::KEY_PAUSE => (ScenicKey::Named(NamedKey::Pause), KeyLocation::Standard),
        Key::KEY_MENU => (
            ScenicKey::Named(NamedKey::ContextMenu),
            KeyLocation::Standard,
        ),
        Key::KEY_LEFTSHIFT => (ScenicKey::Named(NamedKey::Shift), KeyLocation::Left),
        Key::KEY_RIGHTSHIFT => (ScenicKey::Named(NamedKey::Shift), KeyLocation::Right),
        Key::KEY_LEFTCTRL => (ScenicKey::Named(NamedKey::Control), KeyLocation::Left),
        Key::KEY_RIGHTCTRL => (ScenicKey::Named(NamedKey::Control), KeyLocation::Right),
        Key::KEY_LEFTALT => (ScenicKey::Named(NamedKey::Alt), KeyLocation::Left),
        Key::KEY_RIGHTALT => (ScenicKey::Named(NamedKey::AltGraph), KeyLocation::Right),
        Key::KEY_LEFTMETA => (ScenicKey::Named(NamedKey::Super), KeyLocation::Left),
        Key::KEY_RIGHTMETA => (ScenicKey::Named(NamedKey::Super), KeyLocation::Right),
        Key::KEY_F1 => (ScenicKey::Named(NamedKey::F1), KeyLocation::Standard),
        Key::KEY_F2 => (ScenicKey::Named(NamedKey::F2), KeyLocation::Standard),
        Key::KEY_F3 => (ScenicKey::Named(NamedKey::F3), KeyLocation::Standard),
        Key::KEY_F4 => (ScenicKey::Named(NamedKey::F4), KeyLocation::Standard),
        Key::KEY_F5 => (ScenicKey::Named(NamedKey::F5), KeyLocation::Standard),
        Key::KEY_F6 => (ScenicKey::Named(NamedKey::F6), KeyLocation::Standard),
        Key::KEY_F7 => (ScenicKey::Named(NamedKey::F7), KeyLocation::Standard),
        Key::KEY_F8 => (ScenicKey::Named(NamedKey::F8), KeyLocation::Standard),
        Key::KEY_F9 => (ScenicKey::Named(NamedKey::F9), KeyLocation::Standard),
        Key::KEY_F10 => (ScenicKey::Named(NamedKey::F10), KeyLocation::Standard),
        Key::KEY_F11 => (ScenicKey::Named(NamedKey::F11), KeyLocation::Standard),
        Key::KEY_F12 => (ScenicKey::Named(NamedKey::F12), KeyLocation::Standard),
        Key::KEY_F13 => (ScenicKey::Named(NamedKey::F13), KeyLocation::Standard),
        Key::KEY_F14 => (ScenicKey::Named(NamedKey::F14), KeyLocation::Standard),
        Key::KEY_F15 => (ScenicKey::Named(NamedKey::F15), KeyLocation::Standard),
        Key::KEY_F16 => (ScenicKey::Named(NamedKey::F16), KeyLocation::Standard),
        Key::KEY_F17 => (ScenicKey::Named(NamedKey::F17), KeyLocation::Standard),
        Key::KEY_F18 => (ScenicKey::Named(NamedKey::F18), KeyLocation::Standard),
        Key::KEY_F19 => (ScenicKey::Named(NamedKey::F19), KeyLocation::Standard),
        Key::KEY_F20 => (ScenicKey::Named(NamedKey::F20), KeyLocation::Standard),
        Key::KEY_F21 => (ScenicKey::Named(NamedKey::F21), KeyLocation::Standard),
        Key::KEY_F22 => (ScenicKey::Named(NamedKey::F22), KeyLocation::Standard),
        Key::KEY_F23 => (ScenicKey::Named(NamedKey::F23), KeyLocation::Standard),
        Key::KEY_F24 => (ScenicKey::Named(NamedKey::F24), KeyLocation::Standard),
        Key::KEY_MINUS => (ScenicKey::Character('-'), KeyLocation::Standard),
        Key::KEY_EQUAL => (ScenicKey::Character('='), KeyLocation::Standard),
        Key::KEY_LEFTBRACE => (ScenicKey::Character('['), KeyLocation::Standard),
        Key::KEY_RIGHTBRACE => (ScenicKey::Character(']'), KeyLocation::Standard),
        Key::KEY_BACKSLASH => (ScenicKey::Character('\\'), KeyLocation::Standard),
        Key::KEY_SEMICOLON => (ScenicKey::Character(';'), KeyLocation::Standard),
        Key::KEY_APOSTROPHE => (ScenicKey::Character('\''), KeyLocation::Standard),
        Key::KEY_GRAVE => (ScenicKey::Character('`'), KeyLocation::Standard),
        Key::KEY_COMMA => (ScenicKey::Character(','), KeyLocation::Standard),
        Key::KEY_DOT => (ScenicKey::Character('.'), KeyLocation::Standard),
        Key::KEY_SLASH => (ScenicKey::Character('/'), KeyLocation::Standard),
        Key::KEY_KP0 => (ScenicKey::Character('0'), KeyLocation::Numpad),
        Key::KEY_KP1 => (ScenicKey::Character('1'), KeyLocation::Numpad),
        Key::KEY_KP2 => (ScenicKey::Character('2'), KeyLocation::Numpad),
        Key::KEY_KP3 => (ScenicKey::Character('3'), KeyLocation::Numpad),
        Key::KEY_KP4 => (ScenicKey::Character('4'), KeyLocation::Numpad),
        Key::KEY_KP5 => (ScenicKey::Character('5'), KeyLocation::Numpad),
        Key::KEY_KP6 => (ScenicKey::Character('6'), KeyLocation::Numpad),
        Key::KEY_KP7 => (ScenicKey::Character('7'), KeyLocation::Numpad),
        Key::KEY_KP8 => (ScenicKey::Character('8'), KeyLocation::Numpad),
        Key::KEY_KP9 => (ScenicKey::Character('9'), KeyLocation::Numpad),
        Key::KEY_KPDOT => (ScenicKey::Character('.'), KeyLocation::Numpad),
        Key::KEY_KPSLASH => (ScenicKey::Character('/'), KeyLocation::Numpad),
        Key::KEY_KPASTERISK => (ScenicKey::Character('*'), KeyLocation::Numpad),
        Key::KEY_KPMINUS => (ScenicKey::Character('-'), KeyLocation::Numpad),
        Key::KEY_KPPLUS => (ScenicKey::Character('+'), KeyLocation::Numpad),
        Key::KEY_KPEQUAL => (ScenicKey::Character('='), KeyLocation::Numpad),
        Key::KEY_KPENTER => (ScenicKey::Named(NamedKey::Enter), KeyLocation::Numpad),
        _ => return None,
    };
    Some((key, location))
}

fn evdev_key_to_button(key: Key) -> Option<MouseButton> {
    match key {
        Key::BTN_LEFT => Some(MouseButton::Left),
        Key::BTN_RIGHT => Some(MouseButton::Right),
        Key::BTN_MIDDLE => Some(MouseButton::Middle),
        Key::BTN_BACK => Some(MouseButton::Back),
        Key::BTN_FORWARD => Some(MouseButton::Forward),
        _ => None,
    }
}

fn key_to_codepoint(key: ScenicKey, mods: Modifiers, caps_lock: bool) -> Option<char> {
    let shift = mods.shift;
    let uppercase = shift ^ caps_lock;
    match key {
        ScenicKey::Character(ch) => Some(match ch {
            'a'..='z' => {
                if uppercase {
                    ch.to_ascii_uppercase()
                } else {
                    ch
                }
            }
            '0'..='9' => shift_digit(ch, shift)?,
            '-' => {
                if shift {
                    '_'
                } else {
                    '-'
                }
            }
            '=' => {
                if shift {
                    '+'
                } else {
                    '='
                }
            }
            '[' => {
                if shift {
                    '{'
                } else {
                    '['
                }
            }
            ']' => {
                if shift {
                    '}'
                } else {
                    ']'
                }
            }
            '\\' => {
                if shift {
                    '|'
                } else {
                    '\\'
                }
            }
            ';' => {
                if shift {
                    ':'
                } else {
                    ';'
                }
            }
            '\'' => {
                if shift {
                    '"'
                } else {
                    '\''
                }
            }
            '`' => {
                if shift {
                    '~'
                } else {
                    '`'
                }
            }
            ',' => {
                if shift {
                    '<'
                } else {
                    ','
                }
            }
            '.' => {
                if shift {
                    '>'
                } else {
                    '.'
                }
            }
            '/' => {
                if shift {
                    '?'
                } else {
                    '/'
                }
            }
            ' ' => ' ',
            _ => return None,
        }),
        _ => None,
    }
}

fn shift_digit(ch: char, shift: bool) -> Option<char> {
    if !shift {
        return Some(ch);
    }
    Some(match ch {
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU32;
    use std::time::Duration;

    use evdev::{
        AbsInfo, AbsoluteAxisType, AttributeSet, EventType, InputEvent as EvdevInputEvent,
        RelativeAxisType, UinputAbsSetup, uinput::VirtualDevice, uinput::VirtualDeviceBuilder,
    };

    #[test]
    fn key_to_codepoint_respects_shift_and_caps() {
        let mut mods = Modifiers::default();
        mods.shift = false;
        assert_eq!(
            key_to_codepoint(ScenicKey::Character('a'), mods, false),
            Some('a')
        );
        assert_eq!(
            key_to_codepoint(ScenicKey::Character('a'), mods, true),
            Some('A')
        );

        mods.shift = true;
        assert_eq!(
            key_to_codepoint(ScenicKey::Character('a'), mods, false),
            Some('A')
        );
        assert_eq!(
            key_to_codepoint(ScenicKey::Character('a'), mods, true),
            Some('a')
        );
    }

    #[test]
    fn key_to_codepoint_shift_symbols() {
        let mut mods = Modifiers::default();
        mods.shift = true;
        assert_eq!(
            key_to_codepoint(ScenicKey::Character('1'), mods, false),
            Some('!')
        );
        assert_eq!(
            key_to_codepoint(ScenicKey::Character('='), mods, false),
            Some('+')
        );
        assert_eq!(
            key_to_codepoint(ScenicKey::Character('/'), mods, false),
            Some('?')
        );
    }

    #[test]
    fn evdev_key_maps_to_named() {
        let (key, loc) = evdev_key_to_scenic(Key::KEY_LEFTSHIFT).expect("map key");
        assert_eq!(key, ScenicKey::Named(NamedKey::Shift));
        assert_eq!(loc, KeyLocation::Left);
    }

    #[test]
    fn scale_abs_value_maps_range() {
        let state = AbsAxisState {
            value: 50,
            min: 0,
            max: 100,
        };
        assert_eq!(scale_abs_value(state, 101), 50.0);
    }

    #[test]
    fn scale_abs_value_falls_back_to_clamp() {
        let state = AbsAxisState {
            value: 120,
            min: 10,
            max: 10,
        };
        assert_eq!(scale_abs_value(state, 100), 99.0);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn drm_input_reads_uinput_events() {
        let Some((mut vdev, path)) = build_virtual_device() else {
            return;
        };

        let device = match Device::open(&path) {
            Ok(device) => device,
            Err(_) => return,
        };
        set_non_blocking(device.as_raw_fd());
        let (abs_x, abs_y) = init_abs_axes(&device);
        let input_device = InputDevice {
            device,
            abs_x,
            abs_y,
            abs_x_dirty: false,
            abs_y_dirty: false,
            abs_mode: AbsMode::Absolute,
            last_abs_scaled: None,
            touch_active: false,
            touch_tracking: false,
        };

        let input_mask = Arc::new(AtomicU32::new(
            INPUT_MASK_KEY
                | INPUT_MASK_CODEPOINT
                | INPUT_MASK_CURSOR_POS
                | INPUT_MASK_CURSOR_BUTTON,
        ));
        let queue = Arc::new(Mutex::new(InputQueue::new()));
        let cursor_state = Arc::new(Mutex::new(CursorState::new()));
        let mut drm_input = DrmInput {
            devices: vec![input_device],
            cursor_pos: (0.0, 0.0),
            modifiers: Modifiers::default(),
            caps_lock: false,
            screen_size: (100, 50),
            input_mask,
            input_events: Arc::clone(&queue),
            cursor_state,
        };

        let _ = vdev.emit(&[
            EvdevInputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_X.0, 512),
            EvdevInputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_Y.0, 256),
        ]);
        let _ = vdev.emit(&[
            EvdevInputEvent::new(EventType::KEY, Key::KEY_A.0, 1),
            EvdevInputEvent::new(EventType::KEY, Key::BTN_LEFT.0, 1),
        ]);

        drm_input.poll();

        let events = queue.lock().unwrap().drain();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, InputEvent::Key { .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, InputEvent::Codepoint { .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, InputEvent::CursorButton { .. }))
        );

        let cursor_pos = events.iter().find_map(|event| match event {
            InputEvent::CursorPos { x, y } => Some((*x, *y)),
            _ => None,
        });
        let Some((x, y)) = cursor_pos else {
            return;
        };
        let expected_x = scale_abs_value(
            AbsAxisState {
                value: 512,
                min: 0,
                max: 1023,
            },
            100,
        );
        let expected_y = scale_abs_value(
            AbsAxisState {
                value: 256,
                min: 0,
                max: 767,
            },
            50,
        );
        assert!((x - expected_x).abs() < 1.0);
        assert!((y - expected_y).abs() < 1.0);
    }

    #[cfg(target_os = "linux")]
    fn build_virtual_device() -> Option<(VirtualDevice, PathBuf)> {
        let mut keys = AttributeSet::<Key>::new();
        keys.insert(Key::KEY_A);
        keys.insert(Key::BTN_LEFT);

        let mut rel_axes = AttributeSet::<RelativeAxisType>::new();
        rel_axes.insert(RelativeAxisType::REL_X);
        rel_axes.insert(RelativeAxisType::REL_Y);

        let abs_x = UinputAbsSetup::new(AbsoluteAxisType::ABS_X, AbsInfo::new(0, 0, 1023, 0, 0, 0));
        let abs_y = UinputAbsSetup::new(AbsoluteAxisType::ABS_Y, AbsInfo::new(0, 0, 767, 0, 0, 0));

        let builder = match VirtualDeviceBuilder::new() {
            Ok(builder) => builder,
            Err(_) => return None,
        };
        let mut vdev = builder
            .name(&"scenic-drm-test")
            .with_keys(&keys)
            .and_then(|builder| builder.with_relative_axes(&rel_axes))
            .and_then(|builder| builder.with_absolute_axis(&abs_x))
            .and_then(|builder| builder.with_absolute_axis(&abs_y))
            .and_then(|builder| builder.build())
            .ok()?;

        for _ in 0..20 {
            if let Ok(mut nodes) = vdev.enumerate_dev_nodes_blocking() {
                if let Some(Ok(path)) = nodes.next() {
                    return Some((vdev, path));
                }
            }
            std::thread::sleep(Duration::from_millis(25));
        }

        None
    }
}
