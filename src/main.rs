use anyhow::Result;
use dirs::config_dir;

mod app;
mod config;
mod hotkey;
mod message;

fn main() -> Result<()> {
    let config_path = config_dir()
        .map(|p| p.join("FancyTwm").join("config.toml"))
        .unwrap();
    let cfg = config::AppConfig::load(&config_path)?;

    let mut app = app::App::new(cfg)?;
    let _ = app.run();

    Ok(())
}
