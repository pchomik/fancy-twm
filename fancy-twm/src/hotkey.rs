use anyhow::{Context, Result};
use rdev::{Event, EventType, Key, listen};
use std::sync::{Arc, mpsc};
use std::thread;

use crate::config::{AppConfig, Keybinding};

pub struct HotKeysController {
    config: Arc<AppConfig>,
    receiver: mpsc::Receiver<Keybinding>,
}

impl HotKeysController {
    pub fn new(config: Arc<AppConfig>) -> Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let config_for_thread = config.clone();

        thread::spawn(move || {
            let mut pressed_keys = std::collections::HashSet::new();

            let callback = move |event: Event| match event.event_type {
                EventType::KeyPress(key) => {
                    pressed_keys.insert(key);

                    if let Some(matched_binding) = check_match(&pressed_keys, &config_for_thread) {
                        let _ = sender.send(matched_binding);
                    }
                }
                EventType::KeyRelease(key) => {
                    pressed_keys.remove(&key);
                }
                _ => {}
            };

            if let Err(error) = listen(callback) {
                eprintln!("Error starting rdev listener: {:?}", error);
            }
        });

        Ok(Self { config, receiver })
    }

    pub fn register(&mut self) {
        println!("Rdev listener active (Global Hooks)");
    }

    pub fn read(&self) -> Option<Keybinding> {
        self.receiver.try_recv().ok()
    }
}

fn check_match(pressed: &std::collections::HashSet<Key>, config: &AppConfig) -> Option<Keybinding> {
    for kb in &config.keybindings {
        let parts: Vec<&str> = kb.binding.split('+').collect();
        let mut all_match = true;

        for part in parts {
            let key = match part.to_lowercase().as_str() {
                // Modifiers
                "super" | "win" | "meta" | "cmd" | "command" => Key::MetaLeft,
                "metaright" | "winright" | "superright" => Key::MetaRight,
                "alt" => Key::Alt,
                "altgr" => Key::AltGr,
                "ctrl" | "control" | "ctrll" | "ctrlleft" => Key::ControlLeft,
                "ctrlr" | "ctrlright" => Key::ControlRight,
                "shift" | "shiftl" | "shiftleft" => Key::ShiftLeft,
                "shiftr" | "shiftright" => Key::ShiftRight,

                // Letters
                "a" => Key::KeyA,
                "b" => Key::KeyB,
                "c" => Key::KeyC,
                "d" => Key::KeyD,
                "e" => Key::KeyE,
                "f" => Key::KeyF,
                "g" => Key::KeyG,
                "h" => Key::KeyH,
                "i" => Key::KeyI,
                "j" => Key::KeyJ,
                "k" => Key::KeyK,
                "l" => Key::KeyL,
                "m" => Key::KeyM,
                "n" => Key::KeyN,
                "o" => Key::KeyO,
                "p" => Key::KeyP,
                "q" => Key::KeyQ,
                "r" => Key::KeyR,
                "s" => Key::KeyS,
                "t" => Key::KeyT,
                "u" => Key::KeyU,
                "v" => Key::KeyV,
                "w" => Key::KeyW,
                "x" => Key::KeyX,
                "y" => Key::KeyY,
                "z" => Key::KeyZ,

                // Numbers (main row)
                "0" => Key::Num0,
                "1" => Key::Num1,
                "2" => Key::Num2,
                "3" => Key::Num3,
                "4" => Key::Num4,
                "5" => Key::Num5,
                "6" => Key::Num6,
                "7" => Key::Num7,
                "8" => Key::Num8,
                "9" => Key::Num9,

                // Function keys
                "f1" => Key::F1,
                "f2" => Key::F2,
                "f3" => Key::F3,
                "f4" => Key::F4,
                "f5" => Key::F5,
                "f6" => Key::F6,
                "f7" => Key::F7,
                "f8" => Key::F8,
                "f9" => Key::F9,
                "f10" => Key::F10,
                "f11" => Key::F11,
                "f12" => Key::F12,

                // Navigation
                "up" | "uparrow" => Key::UpArrow,
                "down" | "downarrow" => Key::DownArrow,
                "left" | "leftarrow" => Key::LeftArrow,
                "right" | "rightarrow" | "next" => Key::RightArrow,
                "prev" => Key::LeftArrow,
                "home" => Key::Home,
                "end" => Key::End,
                "pageup" | "pgup" => Key::PageUp,
                "pagedown" | "pgdown" => Key::PageDown,
                "insert" | "ins" => Key::Insert,
                "delete" | "del" => Key::Delete,

                // Special keys
                "escape" | "esc" => Key::Escape,
                "return" | "enter" => Key::Return,
                "tab" => Key::Tab,
                "space" | "spacebar" => Key::Space,
                "backspace" | "bksp" => Key::Backspace,
                "capslock" => Key::CapsLock,
                "printscreen" | "prtsc" => Key::PrintScreen,
                "scrolllock" => Key::ScrollLock,
                "pause" => Key::Pause,
                "numlock" => Key::NumLock,

                // Symbols / punctuation
                "backquote" | "grave" | "tilde" => Key::BackQuote,
                "minus" | "dash" | "hyphen" => Key::Minus,
                "equal" | "equals" => Key::Equal,
                "leftbracket" | "lbracket" | "[" => Key::LeftBracket,
                "rightbracket" | "rbracket" | "]" => Key::RightBracket,
                "semicolon" | "semi" => Key::SemiColon,
                "quote" | "apostrophe" => Key::Quote,
                "backslash" | "bslash" => Key::BackSlash,
                "intlbackslash" => Key::IntlBackslash,
                "comma" => Key::Comma,
                "dot" | "period" => Key::Dot,
                "slash" | "forwardslash" => Key::Slash,

                // Keypad
                "kpreturn" | "kpenter" => Key::KpReturn,
                "kpminus" => Key::KpMinus,
                "kpplus" => Key::KpPlus,
                "kpmultiply" | "kpmul" => Key::KpMultiply,
                "kpdivide" | "kpdiv" => Key::KpDivide,
                "kp0" => Key::Kp0,
                "kp1" => Key::Kp1,
                "kp2" => Key::Kp2,
                "kp3" => Key::Kp3,
                "kp4" => Key::Kp4,
                "kp5" => Key::Kp5,
                "kp6" => Key::Kp6,
                "kp7" => Key::Kp7,
                "kp8" => Key::Kp8,
                "kp9" => Key::Kp9,
                "kpdelete" | "kpdecimal" | "kpdot" => Key::KpDelete,
                "function" => Key::Function,

                _ => continue,
            };

            if !pressed.contains(&key) {
                all_match = false;
                break;
            }
        }

        if all_match {
            return Some(kb.clone());
        }
    }
    None
}
