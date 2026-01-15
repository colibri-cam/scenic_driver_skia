use crate::input::{MOD_ALT, MOD_CTRL, MOD_META, MOD_SHIFT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyLocation {
    Standard,
    Left,
    Right,
    Numpad,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedKey {
    Enter,
    Tab,
    Space,
    Escape,
    Backspace,
    Insert,
    Delete,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    PageUp,
    PageDown,
    Home,
    End,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    ContextMenu,
    Shift,
    Control,
    Alt,
    AltGraph,
    Super,
    Meta,
    Hyper,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Character(char),
    Named(NamedKey),
    Unidentified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

pub fn modifiers_to_mask(mods: Modifiers) -> u8 {
    let mut mask = 0;
    if mods.shift {
        mask |= MOD_SHIFT;
    }
    if mods.ctrl {
        mask |= MOD_CTRL;
    }
    if mods.alt {
        mask |= MOD_ALT;
    }
    if mods.meta {
        mask |= MOD_META;
    }
    mask
}

pub fn key_to_scenic(key: Key, location: KeyLocation) -> String {
    match key {
        Key::Character(ch) => character_to_scenic(ch, location),
        Key::Named(named) => named_key_to_scenic(named, location),
        Key::Unidentified => "key_unknown".to_string(),
    }
}

pub fn button_to_scenic(button: MouseButton) -> String {
    match button {
        MouseButton::Left => "btn_left".to_string(),
        MouseButton::Right => "btn_right".to_string(),
        MouseButton::Middle => "btn_middle".to_string(),
        MouseButton::Back | MouseButton::Forward | MouseButton::Other => "btn_unknown".to_string(),
    }
}

fn character_to_scenic(ch: char, location: KeyLocation) -> String {
    if location == KeyLocation::Numpad
        && let Some(name) = numpad_char_to_scenic(ch)
    {
        return name.to_string();
    }

    if let Some(name) = ascii_char_to_scenic(ch) {
        return name.to_string();
    }

    "key_unknown".to_string()
}

fn ascii_char_to_scenic(ch: char) -> Option<&'static str> {
    match ch {
        'a'..='z' => Some(match ch {
            'a' => "key_a",
            'b' => "key_b",
            'c' => "key_c",
            'd' => "key_d",
            'e' => "key_e",
            'f' => "key_f",
            'g' => "key_g",
            'h' => "key_h",
            'i' => "key_i",
            'j' => "key_j",
            'k' => "key_k",
            'l' => "key_l",
            'm' => "key_m",
            'n' => "key_n",
            'o' => "key_o",
            'p' => "key_p",
            'q' => "key_q",
            'r' => "key_r",
            's' => "key_s",
            't' => "key_t",
            'u' => "key_u",
            'v' => "key_v",
            'w' => "key_w",
            'x' => "key_x",
            'y' => "key_y",
            'z' => "key_z",
            _ => return None,
        }),
        'A'..='Z' => ascii_char_to_scenic(ch.to_ascii_lowercase()),
        '0' => Some("key_0"),
        '1' => Some("key_1"),
        '2' => Some("key_2"),
        '3' => Some("key_3"),
        '4' => Some("key_4"),
        '5' => Some("key_5"),
        '6' => Some("key_6"),
        '7' => Some("key_7"),
        '8' => Some("key_8"),
        '9' => Some("key_9"),
        ' ' => Some("key_space"),
        '\'' => Some("key_apostrophe"),
        ',' => Some("key_comma"),
        '-' => Some("key_minus"),
        '.' => Some("key_dot"),
        '/' => Some("key_slash"),
        ';' => Some("key_semicolon"),
        '=' => Some("key_equal"),
        '[' => Some("key_leftbrace"),
        '\\' => Some("key_backslash"),
        ']' => Some("key_rightbrace"),
        '`' => Some("key_grave"),
        _ => None,
    }
}

fn numpad_char_to_scenic(ch: char) -> Option<&'static str> {
    match ch {
        '0' => Some("key_kp0"),
        '1' => Some("key_kp1"),
        '2' => Some("key_kp2"),
        '3' => Some("key_kp3"),
        '4' => Some("key_kp4"),
        '5' => Some("key_kp5"),
        '6' => Some("key_kp6"),
        '7' => Some("key_kp7"),
        '8' => Some("key_kp8"),
        '9' => Some("key_kp9"),
        '.' => Some("key_kpdot"),
        '/' => Some("key_kpslash"),
        '*' => Some("key_kpasterisk"),
        '-' => Some("key_kpminus"),
        '+' => Some("key_kpplus"),
        '=' => Some("key_kpequal"),
        _ => None,
    }
}

fn named_key_to_scenic(key: NamedKey, location: KeyLocation) -> String {
    match key {
        NamedKey::Enter => match location {
            KeyLocation::Numpad => "key_kpenter".to_string(),
            _ => "key_enter".to_string(),
        },
        NamedKey::Tab => "key_tab".to_string(),
        NamedKey::Space => "key_space".to_string(),
        NamedKey::Escape => "key_esc".to_string(),
        NamedKey::Backspace => "key_backspace".to_string(),
        NamedKey::Insert => "key_insert".to_string(),
        NamedKey::Delete => "key_delete".to_string(),
        NamedKey::ArrowLeft => "key_left".to_string(),
        NamedKey::ArrowRight => "key_right".to_string(),
        NamedKey::ArrowUp => "key_up".to_string(),
        NamedKey::ArrowDown => "key_down".to_string(),
        NamedKey::PageUp => "key_pageup".to_string(),
        NamedKey::PageDown => "key_pagedown".to_string(),
        NamedKey::Home => "key_home".to_string(),
        NamedKey::End => "key_end".to_string(),
        NamedKey::CapsLock => "key_capslock".to_string(),
        NamedKey::ScrollLock => "key_scrolllock".to_string(),
        NamedKey::NumLock => "key_numlock".to_string(),
        NamedKey::PrintScreen => "key_screen".to_string(),
        NamedKey::Pause => "key_pause".to_string(),
        NamedKey::ContextMenu => "key_menu".to_string(),
        NamedKey::Shift => modifier_key("key_shift", "key_leftshift", "key_rightshift", location),
        NamedKey::Control => modifier_key("key_ctrl", "key_leftctrl", "key_rightctrl", location),
        NamedKey::Alt => modifier_key("key_alt", "key_leftalt", "key_rightalt", location),
        NamedKey::AltGraph => "key_rightalt".to_string(),
        NamedKey::Super | NamedKey::Meta | NamedKey::Hyper => {
            modifier_key("key_meta", "key_leftmeta", "key_rightmeta", location)
        }
        NamedKey::F1 => "key_f1".to_string(),
        NamedKey::F2 => "key_f2".to_string(),
        NamedKey::F3 => "key_f3".to_string(),
        NamedKey::F4 => "key_f4".to_string(),
        NamedKey::F5 => "key_f5".to_string(),
        NamedKey::F6 => "key_f6".to_string(),
        NamedKey::F7 => "key_f7".to_string(),
        NamedKey::F8 => "key_f8".to_string(),
        NamedKey::F9 => "key_f9".to_string(),
        NamedKey::F10 => "key_f10".to_string(),
        NamedKey::F11 => "key_f11".to_string(),
        NamedKey::F12 => "key_f12".to_string(),
        NamedKey::F13 => "key_f13".to_string(),
        NamedKey::F14 => "key_f14".to_string(),
        NamedKey::F15 => "key_f15".to_string(),
        NamedKey::F16 => "key_f16".to_string(),
        NamedKey::F17 => "key_f17".to_string(),
        NamedKey::F18 => "key_f18".to_string(),
        NamedKey::F19 => "key_f19".to_string(),
        NamedKey::F20 => "key_f20".to_string(),
        NamedKey::F21 => "key_f21".to_string(),
        NamedKey::F22 => "key_f22".to_string(),
        NamedKey::F23 => "key_f23".to_string(),
        NamedKey::F24 => "key_f24".to_string(),
    }
}

fn modifier_key(generic: &str, left: &str, right: &str, location: KeyLocation) -> String {
    match location {
        KeyLocation::Left => left.to_string(),
        KeyLocation::Right => right.to_string(),
        _ => generic.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_to_mask_sets_bits() {
        let mask = modifiers_to_mask(Modifiers {
            shift: true,
            ctrl: true,
            alt: false,
            meta: true,
        });

        assert_eq!(mask, MOD_SHIFT | MOD_CTRL | MOD_META);
    }

    #[test]
    fn key_to_scenic_maps_character() {
        assert_eq!(
            key_to_scenic(Key::Character('a'), KeyLocation::Standard),
            "key_a"
        );
        assert_eq!(
            key_to_scenic(Key::Character('A'), KeyLocation::Standard),
            "key_a"
        );
        assert_eq!(
            key_to_scenic(Key::Character('3'), KeyLocation::Standard),
            "key_3"
        );
    }

    #[test]
    fn key_to_scenic_maps_numpad() {
        assert_eq!(
            key_to_scenic(Key::Character('1'), KeyLocation::Numpad),
            "key_kp1"
        );
        assert_eq!(
            key_to_scenic(Key::Character('.'), KeyLocation::Numpad),
            "key_kpdot"
        );
    }

    #[test]
    fn key_to_scenic_maps_named_keys() {
        assert_eq!(
            key_to_scenic(Key::Named(NamedKey::Enter), KeyLocation::Standard),
            "key_enter"
        );
        assert_eq!(
            key_to_scenic(Key::Named(NamedKey::Enter), KeyLocation::Numpad),
            "key_kpenter"
        );
        assert_eq!(
            key_to_scenic(Key::Named(NamedKey::F5), KeyLocation::Standard),
            "key_f5"
        );
    }

    #[test]
    fn key_to_scenic_maps_named_modifiers() {
        assert_eq!(
            key_to_scenic(Key::Named(NamedKey::AltGraph), KeyLocation::Standard),
            "key_rightalt"
        );
        assert_eq!(
            key_to_scenic(Key::Named(NamedKey::Super), KeyLocation::Standard),
            "key_meta"
        );
        assert_eq!(
            key_to_scenic(Key::Named(NamedKey::Meta), KeyLocation::Standard),
            "key_meta"
        );
    }

    #[test]
    fn key_to_scenic_maps_modifier_location() {
        assert_eq!(
            key_to_scenic(Key::Named(NamedKey::Shift), KeyLocation::Left),
            "key_leftshift"
        );
        assert_eq!(
            key_to_scenic(Key::Named(NamedKey::Shift), KeyLocation::Right),
            "key_rightshift"
        );
        assert_eq!(
            key_to_scenic(Key::Named(NamedKey::Shift), KeyLocation::Standard),
            "key_shift"
        );
    }

    #[test]
    fn key_to_scenic_maps_symbols_and_unknowns() {
        assert_eq!(
            key_to_scenic(Key::Character('-'), KeyLocation::Standard),
            "key_minus"
        );
        assert_eq!(
            key_to_scenic(Key::Unidentified, KeyLocation::Standard),
            "key_unknown"
        );
    }

    #[test]
    fn button_to_scenic_maps_buttons() {
        assert_eq!(button_to_scenic(MouseButton::Left), "btn_left");
        assert_eq!(button_to_scenic(MouseButton::Right), "btn_right");
        assert_eq!(button_to_scenic(MouseButton::Middle), "btn_middle");
        assert_eq!(button_to_scenic(MouseButton::Other), "btn_unknown");
    }
}
