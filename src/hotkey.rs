use anyhow::{Context, Result};
use std::sync::Arc;

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};

use crate::config::{AppConfig, Command, Keybinding};

pub struct HotKeysController {
    config: Arc<AppConfig>,
    manager: GlobalHotKeyManager,
    registered: Vec<(HotKey, Command)>,
}

impl HotKeysController {
    pub fn new(config: Arc<AppConfig>) -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("Failed to initialize global hotkeys")?;
        Ok(Self {
            config,
            manager,
            registered: Vec::new(),
        })
    }

    pub fn register(&mut self) {
        self.config
            .keybindings
            .iter()
            .for_each(|keybinding: &Keybinding| {
                println!("Keybinding to register: {:?}", keybinding);
                let hotkey: HotKey = keybinding.binding.parse().unwrap();
                self.manager
                    .register(hotkey)
                    .context(format!("failed to register key: {}", keybinding.binding))
                    .unwrap();
                self.registered.push((hotkey, keybinding.command.clone()));
                println!("Registered hotkey: {:?}", hotkey)
            });
    }

    pub fn read(&self) -> Option<Command> {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == HotKeyState::Pressed {
                println!("Key Event: {:?}", event);
                if let Some(req) = self.registered.iter().find(|&reg| reg.0.id == event.id) {
                    return Some(req.1.clone());
                }
            }
        }
        None
    }
}
