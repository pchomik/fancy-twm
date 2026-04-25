use crate::config::AppConfig;
// use crate::hotkey::HotKeysController;
use crate::ipc::IpcServerController;
use crate::message::pump_windows_messages;
use crate::tray::TrayController;
use crate::vd::{
    move_active_window_to_next_virtual_desktop, move_active_window_to_prev_virtual_desktop,
    move_active_window_to_virtual_desktop, switch_to_next_virtual_desktop,
    switch_to_prev_virtual_desktop, switch_to_virtual_desktop,
};
// Result allows to return any Error without changing signature.
// Result also allows to use ? for any case.
// Context allows to define custom error message.
use anyhow::{Context, Result};
use fancycore::message::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, time::Duration};

// AtomicBool is needed to keep change unbroken between changes from the thread.
// AtomicBool is easiest solution for simple types like bool.
static RUNNING: AtomicBool = AtomicBool::new(true);

pub struct App {
    pub config: Arc<AppConfig>,
    // pub hotkeys: HotKeysController,
    pub tray: TrayController,
    pub ipc_server: IpcServerController,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self> {
        // Arc allows to have multiple references to single object
        // Arc.clone clones only reference and increment counter. Very cheap operation.
        // Arc is perfect to share variable and still have read and write access.
        let config = Arc::new(config);
        // let hotkeys = HotKeysController::new(config.clone())?;
        let tray = TrayController::new()?;
        let ipc_server = IpcServerController::new()?;
        Ok(Self {
            config,
            tray,
            ipc_server,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        // ctrlc implements CTRL+C signal handler logic
        ctrlc::set_handler(|| {
            RUNNING.store(false, Ordering::SeqCst);
            println!("CTRL+C discovered.");
        })
        .context("Cannot set proper handler")?;

        // self.hotkeys.register();

        println!("Start main loop");
        while RUNNING.load(Ordering::SeqCst) {
            if !pump_windows_messages()? {
                break;
            }

            if let Some(action) = self.ipc_server.read() {
                match action.command {
                    Command::MoveToNextVirtualDesktop => {
                        move_active_window_to_next_virtual_desktop()
                    }
                    Command::MoveToPrevVirtualDesktop => {
                        move_active_window_to_prev_virtual_desktop()
                    }
                    Command::MoveToVirtualDesktop => {
                        if let Some(args) = action.args {
                            move_active_window_to_virtual_desktop(&args[0]);
                        }
                    }
                    Command::SwitchToNextVirtualDesktop => {
                        switch_to_next_virtual_desktop();
                    }
                    Command::SwitchToPrevVirtualDesktop => {
                        switch_to_prev_virtual_desktop();
                    }
                    Command::SwitchToVirtualDesktop => {
                        if let Some(args) = action.args {
                            switch_to_virtual_desktop(&args[0]);
                        }
                    }
                }
            }

            self.tray.read();

            thread::sleep(Duration::from_millis(25));
        }
        println!("Exit main loop");
        Ok(())
    }
}
