use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub keybindings: Vec<Keybinding>,
    pub virtual_desktops: Vec<VirtualDesktop>,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).context("Read file failed")?;
        let config: AppConfig = toml::from_str(&content).context("Parse file failed")?;
        println!("{:?}", config);
        Ok(config)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum Command {
    MoveToNextVirtualDesktop,
    MoveToPrevVirtualDesktop,
    MoveToVirtualDesktop,
    ChangeToNextVirtualDesktop,
    ChangeToPrevVirtualDesktop,
    ChangeToVirtualDesktop,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Keybinding {
    pub binding: String,
    pub command: Command,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VirtualDesktop {
    pub name: String,
    pub workspaces: Vec<Workspace>,
}

#[derive(Debug, Deserialize, Clone)]
pub enum Layout {
    Monocle,
    Columns,
    Rows,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Workspace {
    pub layout: Layout,
}
