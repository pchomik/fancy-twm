use anyhow::{Context, Result};
use std::sync::Arc;

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};

use crate::config::AppConfig;

pub struct HotKeysController {
    config: Arc<AppConfig>,
    manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
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
        self.config.keybindings.iter().for_each(|keybinding| {
            println!("{:?}", keybinding);
            let hotkey: HotKey = keybinding.binding.parse().unwrap();
            self.manager
                .register(hotkey)
                .context(format!("failed to register key: {}", keybinding.binding))
                .unwrap();
            self.registered.push(hotkey);
        });
    }

    pub fn read(&self) {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == HotKeyState::Pressed {
                println!("{:?}", event);
            }
        }
    }
}
