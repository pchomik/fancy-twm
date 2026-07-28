use crate::config::AppConfig;
use crate::border::BorderOverlay;
use crate::ipc::IpcServerController;
use crate::platform::{VirtualDesktopTracker, pump_windows_messages};
use crate::tiling::{MoveDir, TilingEngine};
use crate::tracker::WindowTracker;
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
use tilo_core::message::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use std::{thread, time::Duration};

// AtomicBool is needed to keep change unbroken between changes from the thread.
// AtomicBool is easiest solution for simple types like bool.
static RUNNING: AtomicBool = AtomicBool::new(true);

pub struct App {
    pub config: Arc<AppConfig>,
    pub tray: TrayController,
    pub ipc_server: IpcServerController,
    vd_tracker: VirtualDesktopTracker,
    window_tracker: WindowTracker,
    tiling: TilingEngine,
    border: Option<BorderOverlay>,
    last_position_check: Instant,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self> {
        // Arc allows to have multiple references to single object
        // Arc.clone clones only reference and increment counter. Very cheap operation.
        // Arc is perfect to share variable and still have read and write access.
        let config = Arc::new(config);
        let tray = TrayController::new()?;
        let ipc_server = IpcServerController::new()?;

        let vd_tracker = VirtualDesktopTracker::new()?;
        let window_tracker = WindowTracker::new(&config)?;
        let tiling = TilingEngine::new(&config, vd_tracker.current_index())?;
        let border = BorderOverlay::new(&config.window_border)?;

        Ok(Self {
            config,
            tray,
            ipc_server,
            vd_tracker,
            window_tracker,
            tiling,
            border,
            last_position_check: Instant::now(),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        // ctrlc implements CTRL+C signal handler logic
        ctrlc::set_handler(|| {
            RUNNING.store(false, Ordering::SeqCst);
            println!("CTRL+C discovered.");
        })
        .context("Cannot set proper handler")?;

        // Initial tiling pass.
        self.retile();

        println!("Start main loop");
        while RUNNING.load(Ordering::SeqCst) {
            if !pump_windows_messages()? {
                break;
            }

            // Detect virtual desktop changes.
            if let Some(new_vd) = self.vd_tracker.check_for_changes() {
                crate::log!("app: VD changed to {}", new_vd);
                if let Some(border) = &self.border {
                    border.hide();
                }
                self.tiling.on_vd_changed(&self.config, new_vd);
                // Refresh the tracker for the new VD before re-tiling so we
                // don't apply the new layout to the previous VD's windows.
                self.window_tracker.refresh();
                self.retile();
            }

            // Poll window tracker (WinEvent hooks + periodic scan).
            if self.window_tracker.poll() {
                self.retile();
            }

            // Update active-window border overlay.
            if let Some(border) = &mut self.border {
                border.update(self.window_tracker.is_moving());
            }

            // Periodic position verification & correction.
            if self.config.periodic_check.enabled {
                let interval = Duration::from_millis(self.config.periodic_check.interval_ms);
                if self.last_position_check.elapsed() >= interval {
                    self.tiling
                        .verify_positions(self.config.periodic_check.tolerance);
                    self.last_position_check = Instant::now();
                }
            }

            if let Some(action) = self.ipc_server.read() {
                if let Some(border) = &self.border {
                    border.hide();
                }
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
                    Command::RetileActiveMonitor => {
                        self.retile();
                    }
                    Command::RetileVirtualDesktop => {
                        self.retile();
                    }
                    Command::MoveWindowRight => {
                        if self.tiling.move_focused(MoveDir::Right) {
                            self.rapid_verify_after_move();
                        }
                    }
                    Command::MoveWindowLeft => {
                        if self.tiling.move_focused(MoveDir::Left) {
                            self.rapid_verify_after_move();
                        }
                    }
                    Command::MoveWindowUp => {
                        if self.tiling.move_focused(MoveDir::Up) {
                            self.rapid_verify_after_move();
                        }
                    }
                    Command::MoveWindowDown => {
                        if self.tiling.move_focused(MoveDir::Down) {
                            self.rapid_verify_after_move();
                        }
                    }
                    Command::CycleLayout => {
                        self.tiling.cycle_layout(&self.config);
                    }
                    Command::SetLayout => {
                        if let Some(args) = action.args {
                            if let Some(layout_name) = args.first() {
                                self.tiling.set_layout(layout_name, &self.config);
                            }
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

    /// Recomputes and applies tiling for all monitors on the current VD.
    fn retile(&mut self) {
        let tracked = self.window_tracker.windows().to_vec();
        self.tiling.retile(&tracked);
    }

    /// Rapidly verifies window positions after a cross-monitor move.
    ///
    /// When a window moves between monitors with different DPI, the target
    /// application may receive `WM_DPICHANGED` and resize itself. This method
    /// performs several rapid position checks over ~200ms to catch and correct
    /// any drift caused by the target window's DPI handling.
    fn rapid_verify_after_move(&mut self) {
        // Perform 8 checks over ~200ms (25ms intervals) to catch DPI-induced
        // resizes from the target window's WM_DPICHANGED handler.
        for _ in 0..8 {
            thread::sleep(Duration::from_millis(25));
            self.tiling.verify_positions(self.config.periodic_check.tolerance);
        }
    }
}
