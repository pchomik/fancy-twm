use crate::config::AppConfig;
use crate::hotkey::HotKeysController;
use crate::message::pump_windows_messages;
// Result allows to return any Error without changing signature.
// Result also allows to use ? for any case.
// Context allows to define custom error message.
use anyhow::{Context, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, time::Duration};

// AtomicBool is needed to keep change unbroken between changes from the thread.
// AtomicBool is easiest solution for simple types like bool.
static RUNNING: AtomicBool = AtomicBool::new(true);

pub struct App {
    pub config: Arc<AppConfig>,
    pub hotkeys: HotKeysController,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self> {
        // Arc allows to have multiple references to single object
        // Arc.clone clones only reference and increment counter. Very cheap operation.
        // Arc is perfect to share variable and still have read and write access.
        let config = Arc::new(config);
        let hotkeys = HotKeysController::new(config.clone())?;
        Ok(Self { config, hotkeys })
    }

    pub fn run(&mut self) -> Result<()> {
        // ctrlc implements CTRL+C signal handler logic
        ctrlc::set_handler(|| {
            RUNNING.store(false, Ordering::SeqCst);
            println!("CTRL+C discovered.");
        })
        .context("Cannot set proper handler")?;

        self.hotkeys.register();

        println!("Start main loop");
        while RUNNING.load(Ordering::SeqCst) {
            if !pump_windows_messages()? {
                break;
            }

            self.hotkeys.read();

            thread::sleep(Duration::from_millis(25));
        }
        println!("Exit main loop");
        Ok(())
    }
}
