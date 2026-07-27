#![windows_subsystem = "windows"]

use anyhow::Result;
use dirs::home_dir;

mod app;
mod border;
mod config;
mod grid;
mod ipc;
mod layout;
mod log;
mod platform;
mod position;
mod tiling;
mod tiling_state;
mod tracker;
mod tray;
mod vd;

fn main() -> Result<()> {
    // Optional file logging (set TILOSRV_LOG=1 to enable). Must be called
    // before any logging happens.
    log::init();

    // Must be called before any window is created or positioned. Prevents
    // Windows from rescaling windows when they cross monitor DPI boundaries,
    // eliminating the visible double-resize during cross-monitor moves.
    platform::set_process_dpi_awareness();

    let config_path = home_dir()
        .map(|p| p.join(".config").join("tilo").join("config.toml"))
        .unwrap();
    let cfg = config::AppConfig::load(&config_path)?;

    let mut app = app::App::new(cfg)?;
    let _ = app.run();

    Ok(())
}
