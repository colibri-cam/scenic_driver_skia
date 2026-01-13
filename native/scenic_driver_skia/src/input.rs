use std::collections::VecDeque;

use rustler::{Atom, Encoder, Env, LocalPid, OwnedEnv, Term};

#[derive(Clone, Debug)]
pub enum InputEvent {
    Key {
        key: String,
        action: u8,
        mods: u8,
    },
    Codepoint {
        codepoint: char,
        mods: u8,
    },
    CursorPos {
        x: f32,
        y: f32,
    },
    CursorButton {
        button: String,
        action: u8,
        mods: u8,
        x: f32,
        y: f32,
    },
    CursorScroll {
        dx: f32,
        dy: f32,
        x: f32,
        y: f32,
    },
    Viewport {
        entered: bool,
        x: f32,
        y: f32,
    },
    ViewportReshape {
        width: u32,
        height: u32,
    },
}

pub const INPUT_MASK_KEY: u32 = 0x01;
pub const INPUT_MASK_CODEPOINT: u32 = 0x02;
pub const INPUT_MASK_CURSOR_POS: u32 = 0x04;
pub const INPUT_MASK_CURSOR_BUTTON: u32 = 0x08;
pub const INPUT_MASK_CURSOR_SCROLL: u32 = 0x10;
pub const INPUT_MASK_VIEWPORT: u32 = 0x20;

pub const MOD_SHIFT: u8 = 0x01;
pub const MOD_CTRL: u8 = 0x02;
pub const MOD_ALT: u8 = 0x04;
pub const MOD_META: u8 = 0x08;

pub const ACTION_PRESS: u8 = 1;
pub const ACTION_RELEASE: u8 = 0;

rustler::atoms! {
    key,
    codepoint,
    cursor_pos,
    cursor_button,
    cursor_scroll,
    viewport,
    enter,
    exit,
    reshape,
    shift,
    ctrl,
    alt,
    meta,
    input_ready
}

pub struct InputQueue {
    events: VecDeque<InputEvent>,
    target: Option<LocalPid>,
    notified: bool,
}

impl InputQueue {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            target: None,
            notified: false,
        }
    }

    pub fn push_event(&mut self, event: InputEvent) -> Option<LocalPid> {
        self.events.push_back(event);
        if self.notified {
            return None;
        }
        let target = self.target?;
        self.notified = true;
        Some(target)
    }

    pub fn set_target(&mut self, target: Option<LocalPid>) -> Option<LocalPid> {
        self.target = target;
        if self.notified {
            return None;
        }
        if self.events.is_empty() {
            return None;
        }
        let target = self.target?;
        self.notified = true;
        Some(target)
    }

    pub fn drain(&mut self) -> Vec<InputEvent> {
        self.notified = false;
        self.events.drain(..).collect()
    }
}

pub fn notify_input_ready(pid: LocalPid) {
    let mut env = OwnedEnv::new();
    let _ = env.send_and_clear(&pid, |_| input_ready());
}

impl InputEvent {
    fn mods_to_terms<'a>(env: Env<'a>, mods: u8) -> Vec<Term<'a>> {
        let mut terms = Vec::new();
        if mods & MOD_SHIFT != 0 {
            terms.push(shift().encode(env));
        }
        if mods & MOD_CTRL != 0 {
            terms.push(ctrl().encode(env));
        }
        if mods & MOD_ALT != 0 {
            terms.push(alt().encode(env));
        }
        if mods & MOD_META != 0 {
            terms.push(meta().encode(env));
        }
        terms
    }
}

impl Encoder for InputEvent {
    fn encode<'a>(&self, env: Env<'a>) -> Term<'a> {
        match self {
            InputEvent::Key {
                key: key_name,
                action,
                mods,
            } => {
                let key_atom = Atom::from_str(env, key_name)
                    .unwrap_or_else(|_| Atom::from_str(env, "key_unknown").expect("key_unknown"));
                let mods = InputEvent::mods_to_terms(env, *mods);
                (key(), (key_atom, *action, mods)).encode(env)
            }
            InputEvent::Codepoint {
                codepoint: codepoint_char,
                mods,
            } => {
                let mods = InputEvent::mods_to_terms(env, *mods);
                (codepoint(), (codepoint_char.to_string(), mods)).encode(env)
            }
            InputEvent::CursorPos { x, y } => (cursor_pos(), (*x, *y)).encode(env),
            InputEvent::CursorButton {
                button: button_name,
                action,
                mods,
                x,
                y,
            } => {
                let button_atom = Atom::from_str(env, button_name)
                    .unwrap_or_else(|_| Atom::from_str(env, "btn_unknown").expect("btn_unknown"));
                let mods = InputEvent::mods_to_terms(env, *mods);
                (cursor_button(), (button_atom, *action, mods, (*x, *y))).encode(env)
            }
            InputEvent::CursorScroll { dx, dy, x, y } => {
                (cursor_scroll(), ((*dx, *dy), (*x, *y))).encode(env)
            }
            InputEvent::Viewport { entered, x, y } => {
                let dir = if *entered { enter() } else { exit() };
                (viewport(), (dir, (*x, *y))).encode(env)
            }
            InputEvent::ViewportReshape { width, height } => {
                (viewport(), (reshape(), (*width, *height))).encode(env)
            }
        }
    }
}
