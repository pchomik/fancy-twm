//! Application configuration schema.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub virtual_desktops: Vec<VirtualDesktop>,

    /// Global grid definition shared by all monitors.
    #[serde(default)]
    pub grid: GridConfig,

    /// Windows matching any of these rules are not tracked/tiled.
    #[serde(default)]
    pub ignore: Vec<IgnoreRule>,

    /// Periodic full window scan (drives window add/remove detection as a
    /// fallback/complement to the WinEvent hook).
    #[serde(default)]
    pub scan: ScanConfig,

    /// Periodic window position verification & correction.
    #[serde(default)]
    pub periodic_check: PeriodicCheckConfig,

    /// Order in which layouts are cycled when the CycleLayout command is used.
    /// Uses layout names: "Monocle", "Columns", "Rows".
    #[serde(default = "default_cycle_order")]
    pub cycle_order: Vec<Layout>,
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
pub struct VirtualDesktop {
    pub name: String,
    #[serde(default)]
    pub monitors: Vec<MonitorLayout>,
}

/// Layout configuration for a single monitor within a virtual desktop.
#[derive(Debug, Deserialize, Clone)]
pub struct MonitorLayout {
    /// Optional monitor device name (e.g. `\\.\DISPLAY1`). When omitted the
    /// layout applies to monitors by position (index) left-to-right.
    #[serde(default)]
    pub monitor: Option<String>,
    pub layout: Layout,
    /// Maximum number of column areas (Columns layout).
    #[serde(default)]
    pub max_columns: Option<usize>,
    /// Maximum number of row areas (Rows layout).
    #[serde(default)]
    pub max_rows: Option<usize>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Monocle,
    Columns,
    Rows,
    Grid,
}

/// Global grid of cells used to compute window rectangles.
#[derive(Debug, Deserialize, Clone, Copy)]
pub struct GridConfig {
    #[serde(default = "default_rows")]
    pub rows: usize,
    #[serde(default = "default_columns")]
    pub columns: usize,
    /// Gap in pixels between cells and around the work area.
    #[serde(default)]
    pub gap: i32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            rows: default_rows(),
            columns: default_columns(),
            gap: 0,
        }
    }
}

fn default_rows() -> usize {
    4
}

fn default_columns() -> usize {
    4
}

/// A window is ignored when ANY specified field's regex matches.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct IgnoreRule {
    /// Regex matched against the process name (e.g. `explorer.exe`).
    #[serde(default)]
    pub process: Option<String>,
    /// Regex matched against the window title.
    #[serde(default)]
    pub title: Option<String>,
}

/// Configuration for the periodic full window scan.
#[derive(Debug, Deserialize, Clone, Copy)]
pub struct ScanConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_scan_interval")]
    pub interval_ms: u64,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: default_scan_interval(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_scan_interval() -> u64 {
    1000
}

/// Configuration for the periodic position verification & correction.
#[derive(Debug, Deserialize, Clone, Copy)]
pub struct PeriodicCheckConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_scan_interval")]
    pub interval_ms: u64,
    /// Allowed pixel deviation before a window is considered mispositioned.
    #[serde(default = "default_tolerance")]
    pub tolerance: i32,
}

impl Default for PeriodicCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: default_scan_interval(),
            tolerance: default_tolerance(),
        }
    }
}

fn default_tolerance() -> i32 {
    5
}

fn default_cycle_order() -> Vec<Layout> {
    vec![Layout::Monocle, Layout::Columns, Layout::Rows, Layout::Grid]
}
