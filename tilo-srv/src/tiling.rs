//! Tiling engine — assigns tracked windows to layout areas on each monitor and
//! applies the computed geometry.
//!
//! # Assignment model
//!
//! Each virtual-desktop/monitor pair keeps ordered areas. Empty areas preserve
//! holes created by manual movement. The final area accepts multiple windows;
//! all stacked windows share its rectangle and the tiler never changes z-order.
//!
//! This naturally satisfies the shift rules:
//! - A new window is inserted at the front (first area); existing windows shift
//!   toward higher areas. Windows already in the last area stay there.
//! - When a window is removed normally, remaining windows compact toward the
//!   first area.

use crate::config::{AppConfig, Layout};
use crate::grid::Grid;
use crate::layout::{ColumnsLayout, GridLayout, MonocleLayout, RowsLayout, TilingLayout};
use crate::platform::{self, HWND, MonitorInfo, Rect};
use crate::position;
pub use crate::tiling_state::MoveDir;
use crate::tiling_state::{AreaState, MoveTarget, MovementLayout, resolve_move};
use std::collections::HashMap;

/// Geometry for a single physical monitor.
struct MonitorTiler {
    monitor: MonitorInfo,
}

impl MonitorTiler {
    fn new(monitor: MonitorInfo) -> Self {
        Self { monitor }
    }
}

/// Orchestrates tiling across all monitors for the current virtual desktop.
pub struct TilingEngine {
    grid: Grid,
    tilers: Vec<MonitorTiler>,
    /// Window assignment keyed by (virtual desktop index, monitor index).
    states: HashMap<(usize, usize), AreaState<HWND>>,
    /// Layout selection per monitor index (resolved from config for the active VD).
    layouts: Vec<LayoutSpec>,
    /// Runtime layout overrides keyed by (vd_index, monitor_index).
    /// These persist across VD switches and are only reset on app restart.
    layout_overrides: HashMap<(usize, usize), LayoutSpec>,
    /// Current virtual desktop index (needed for override lookup).
    current_vd: usize,
}

/// A resolved layout for one monitor.
#[derive(Clone, Debug)]
enum LayoutSpec {
    Monocle,
    Columns { max_columns: usize },
    Rows { max_rows: usize },
    Grid,
}

impl LayoutSpec {
    /// Returns the config `Layout` type this spec corresponds to.
    fn to_layout_type(&self) -> Layout {
        match self {
            LayoutSpec::Monocle => Layout::Monocle,
            LayoutSpec::Columns { .. } => Layout::Columns,
            LayoutSpec::Rows { .. } => Layout::Rows,
            LayoutSpec::Grid => Layout::Grid,
        }
    }

    fn build(&self) -> Box<dyn TilingLayout> {
        match self {
            LayoutSpec::Monocle => Box::new(MonocleLayout),
            LayoutSpec::Columns { max_columns } => Box::new(ColumnsLayout {
                max_columns: *max_columns,
            }),
            LayoutSpec::Rows { max_rows } => Box::new(RowsLayout {
                max_rows: *max_rows,
            }),
            LayoutSpec::Grid => Box::new(GridLayout),
        }
    }

    fn movement_layout(&self, grid: &Grid) -> MovementLayout {
        match self {
            LayoutSpec::Monocle => MovementLayout::Monocle,
            LayoutSpec::Columns { .. } => MovementLayout::Columns,
            LayoutSpec::Rows { .. } => MovementLayout::Rows,
            LayoutSpec::Grid => MovementLayout::Grid {
                columns: grid.columns,
            },
        }
    }
}

impl TilingEngine {
    /// Creates the engine from configuration, enumerating monitors and
    /// resolving layouts for the given virtual desktop index.
    pub fn new(config: &AppConfig, vd_index: usize) -> anyhow::Result<Self> {
        let grid = Grid::from_config(&config.grid);
        let monitors = platform::enum_monitors()?;

        let layouts = monitors
            .iter()
            .enumerate()
            .map(|(i, _)| resolve_layout(config, vd_index, i))
            .collect();

        let tilers = monitors.into_iter().map(MonitorTiler::new).collect();

        Ok(Self {
            grid,
            tilers,
            states: HashMap::new(),
            layouts,
            layout_overrides: HashMap::new(),
            current_vd: vd_index,
        })
    }

    /// Re-resolves layouts after a virtual desktop change.
    pub fn on_vd_changed(&mut self, config: &AppConfig, vd_index: usize) {
        crate::log!("on_vd_changed: vd {} -> {}", self.current_vd, vd_index);
        self.current_vd = vd_index;
        self.layouts = (0..self.tilers.len())
            .map(|i| self.resolve_layout_with_override(config, vd_index, i))
            .collect();
        for (i, spec) in self.layouts.iter().enumerate() {
            crate::log!("  monitor {} layout = {:?}", i, spec);
        }
    }

    /// Recomputes and applies window positions for all monitors.
    ///
    /// `tracked` is the set of tracked windows on the current VD (all monitors).
    pub fn retile(&mut self, tracked: &[HWND]) {
        crate::log!(
            "retile: {} tracked windows: {:?}",
            tracked.len(),
            tracked.iter().map(|w| w.0 as usize).collect::<Vec<_>>()
        );

        // Group tracked windows by monitor.
        let mut per_monitor: Vec<Vec<HWND>> = vec![Vec::new(); self.tilers.len()];
        for &hwnd in tracked {
            if let Some(idx) = self.monitor_index_for(hwnd) {
                per_monitor[idx].push(hwnd);
            }
        }

        for (i, present) in per_monitor.iter().enumerate() {
            let spec = &self.layouts[i];
            let num_areas = spec.build().areas(&self.grid).len().max(1);
            crate::log!(
                "retile: monitor {} spec={:?} num_areas={} present={:?}",
                i,
                spec,
                num_areas,
                present.iter().map(|w| w.0 as usize).collect::<Vec<_>>()
            );
            let state = self
                .states
                .entry((self.current_vd, i))
                .or_insert_with(|| AreaState::new(num_areas));
            state.resize_and_compact(num_areas);
            state.sync_present(present);
        }

        self.apply();
    }

    /// Applies the current assignment to the screen.
    ///
    /// Every normally-sized tracked window is positioned; maximized and
    /// fullscreen windows retain their current geometry but remain assigned.
    /// Windows beyond the last area all share the last area's rectangle,
    /// stacked on top of each other — the topmost in z-order is the visible
    /// one, and z-order is left untouched (managed by Windows / the user).
    ///
    /// Gaps are scaled per-monitor based on DPI so they appear visually
    /// consistent across monitors with different scaling factors.
    fn apply(&self) {
        for (monitor_index, (tiler, spec)) in
            self.tilers.iter().zip(self.layouts.iter()).enumerate()
        {
            let dpi = platform::get_monitor_dpi(tiler.monitor.handle);
            let grid = self.grid.with_scaled_gap(dpi);
            let layout = spec.build();
            let areas = layout.areas(&grid);
            let work = tiler.monitor.work_area;

            let Some(state) = self.states.get(&(self.current_vd, monitor_index)) else {
                continue;
            };
            for (area_index, windows) in state.areas().iter().enumerate() {
                let Some(layout_area) = areas.get(area_index) else {
                    continue;
                };
                let rect = position::calculate_window_rect(layout_area, &grid, &work);
                for hwnd in windows {
                    let geometry_preserved = platform::is_window_maximized_or_fullscreen(*hwnd);
                    if !geometry_preserved && let Some(rect) = rect {
                        platform::set_window_pos(*hwnd, rect);
                    }
                    crate::log!(
                        "apply: hwnd={} monitor={} area={} rect={:?} geometry_preserved={}",
                        hwnd.0 as usize,
                        monitor_index,
                        area_index,
                        rect,
                        geometry_preserved
                    );
                }
            }
        }
    }

    /// Finds the monitor index a window currently belongs to.
    fn monitor_index_for(&self, hwnd: HWND) -> Option<usize> {
        let mon = platform::get_monitor_for_window(hwnd)?;
        self.tilers
            .iter()
            .position(|t| t.monitor.handle == mon.handle)
    }

    /// Returns the area index a window currently occupies on its monitor.
    fn area_of(&self, hwnd: HWND) -> Option<(usize, usize)> {
        (0..self.tilers.len()).find_map(|monitor_index| {
            self.states
                .get(&(self.current_vd, monitor_index))
                .and_then(|state| state.area_of(hwnd))
                .map(|area| (monitor_index, area))
        })
    }

    /// Moves the focused window according to its active layout.
    pub fn move_focused(&mut self, dir: MoveDir) -> bool {
        let Some(hwnd) = platform::get_foreground_window() else {
            return false;
        };
        let Some((monitor_index, area)) = self.area_of(hwnd) else {
            return false;
        };

        let spec = &self.layouts[monitor_index];
        let area_count = spec.build().areas(&self.grid).len().max(1);
        match resolve_move(spec.movement_layout(&self.grid), area, area_count, dir) {
            MoveTarget::None => false,
            MoveTarget::Area(target) => {
                let Some(state) = self.states.get_mut(&(self.current_vd, monitor_index)) else {
                    return false;
                };
                if !state.move_within(hwnd, target) {
                    return false;
                }
                self.apply();
                true
            }
            MoveTarget::PreviousMonitorLast => {
                self.move_to_monitor(hwnd, monitor_index, monitor_index.checked_sub(1), true)
            }
            MoveTarget::NextMonitorFirst => {
                self.move_to_monitor(hwnd, monitor_index, Some(monitor_index + 1), false)
            }
        }
    }

    /// Moves a window to a target monitor (first or last area).
    ///
    /// The window is moved directly to its final position on the target
    /// monitor in a single `SetWindowPos` call (via `apply()`), avoiding
    /// the double-resize that occurs when using an intermediate placeholder.
    fn move_to_monitor(
        &mut self,
        hwnd: HWND,
        from: usize,
        to: Option<usize>,
        to_last: bool,
    ) -> bool {
        let Some(to) = to else {
            return false;
        };
        if to >= self.tilers.len() {
            return false;
        }

        let source_key = (self.current_vd, from);
        let Some(source) = self.states.get_mut(&source_key) else {
            return false;
        };
        if !source.remove_preserve(hwnd) {
            return false;
        }

        let target_area_count = self.layouts[to].build().areas(&self.grid).len().max(1);
        let target = self
            .states
            .entry((self.current_vd, to))
            .or_insert_with(|| AreaState::new(target_area_count));
        target.resize_and_compact(target_area_count);
        if to_last {
            target.insert_last(hwnd);
        } else {
            target.insert_first(hwnd);
        }

        self.apply();
        true
    }

    /// Sets a specific layout for the monitor containing the focused window.
    ///
    /// The layout name is matched case-insensitively against: Monocle, Columns,
    /// Rows, Grid. The override is stored per (vd_index, monitor_index) and
    /// persists until the application restarts.
    pub fn set_layout(&mut self, layout_name: &str, config: &AppConfig) -> bool {
        let Some(hwnd) = platform::get_foreground_window() else {
            return false;
        };
        let Some((mi, _)) = self.area_of(hwnd) else {
            return false;
        };

        // Parse layout name case-insensitively.
        let layout = match layout_name.to_lowercase().as_str() {
            "monocle" => Layout::Monocle,
            "columns" => Layout::Columns,
            "rows" => Layout::Rows,
            "grid" => Layout::Grid,
            _ => return false,
        };

        // Build LayoutSpec from the config's monitor settings.
        let spec = build_layout_spec(config, self.current_vd, mi, layout);

        // Store override and apply.
        self.layout_overrides
            .insert((self.current_vd, mi), spec.clone());
        self.layouts[mi] = spec;
        let area_count = self.layouts[mi].build().areas(&self.grid).len().max(1);
        if let Some(state) = self.states.get_mut(&(self.current_vd, mi)) {
            state.redistribute(area_count);
        }
        self.apply();
        true
    }

    /// Cycles the layout of the monitor containing the focused window.
    ///
    /// The cycle order is defined by `config.cycle_order`. The current layout
    /// is looked up in the cycle order and advanced to the next entry (wrapping
    /// around). The override is stored per (vd_index, monitor_index) and
    /// persists until the application restarts.
    pub fn cycle_layout(&mut self, config: &AppConfig) -> bool {
        let Some(hwnd) = platform::get_foreground_window() else {
            return false;
        };
        let Some((mi, _)) = self.area_of(hwnd) else {
            return false;
        };

        let cycle_order = &config.cycle_order;
        if cycle_order.is_empty() {
            return false;
        }

        // Determine the current layout type (from override or resolved config).
        let current_layout_type = self.layouts[mi].to_layout_type();

        // Find current position in cycle order and advance.
        let current_pos = cycle_order.iter().position(|l| *l == current_layout_type);
        let next_pos = match current_pos {
            Some(pos) => (pos + 1) % cycle_order.len(),
            None => 0, // If current layout not in cycle order, start from beginning.
        };
        let next_layout = cycle_order[next_pos];

        // Build LayoutSpec from the config's monitor settings for max_columns/max_rows.
        let spec = build_layout_spec(config, self.current_vd, mi, next_layout);

        // Store override and apply.
        self.layout_overrides
            .insert((self.current_vd, mi), spec.clone());
        self.layouts[mi] = spec;
        let area_count = self.layouts[mi].build().areas(&self.grid).len().max(1);
        if let Some(state) = self.states.get_mut(&(self.current_vd, mi)) {
            state.redistribute(area_count);
        }
        self.apply();
        true
    }

    /// Resolves layout for a monitor, checking runtime overrides first.
    fn resolve_layout_with_override(
        &self,
        config: &AppConfig,
        vd_index: usize,
        monitor_index: usize,
    ) -> LayoutSpec {
        if let Some(spec) = self.layout_overrides.get(&(vd_index, monitor_index)) {
            return spec.clone();
        }
        resolve_layout(config, vd_index, monitor_index)
    }

    /// Verifies normally-sized window positions and corrects any that drifted
    /// beyond `tolerance`. Maximized and fullscreen windows remain untouched.
    /// Returns the number of windows corrected.
    pub fn verify_positions(&self, tolerance: i32) -> usize {
        let mut corrected = 0;

        for (monitor_index, (tiler, spec)) in
            self.tilers.iter().zip(self.layouts.iter()).enumerate()
        {
            let dpi = platform::get_monitor_dpi(tiler.monitor.handle);
            let grid = self.grid.with_scaled_gap(dpi);
            let layout = spec.build();
            let areas = layout.areas(&grid);
            let work = tiler.monitor.work_area;

            let Some(state) = self.states.get(&(self.current_vd, monitor_index)) else {
                continue;
            };
            for (area_index, windows) in state.areas().iter().enumerate() {
                let Some(layout_area) = areas.get(area_index) else {
                    continue;
                };
                let Some(expected) = position::calculate_window_rect(layout_area, &grid, &work)
                else {
                    continue;
                };
                for hwnd in windows {
                    if platform::is_window_maximized_or_fullscreen(*hwnd) {
                        continue;
                    }
                    let Some(actual) = platform::get_visible_window_rect(*hwnd) else {
                        continue;
                    };
                    if rects_differ(&actual, &expected, tolerance) {
                        platform::set_window_pos(*hwnd, expected);
                        corrected += 1;
                    }
                }
            }
        }

        corrected
    }
}

/// Resolves the layout for a monitor within a virtual desktop from config.
///
/// Falls back to `Monocle` when no matching configuration exists.
fn resolve_layout(config: &AppConfig, vd_index: usize, monitor_index: usize) -> LayoutSpec {
    let Some(vd) = config.virtual_desktops.get(vd_index) else {
        return LayoutSpec::Monocle;
    };

    let Some(ml) = vd.monitors.get(monitor_index) else {
        return LayoutSpec::Monocle;
    };

    match ml.layout {
        Layout::Monocle => LayoutSpec::Monocle,
        Layout::Columns => LayoutSpec::Columns {
            max_columns: ml.max_columns.unwrap_or(2),
        },
        Layout::Rows => LayoutSpec::Rows {
            max_rows: ml.max_rows.unwrap_or(2),
        },
        Layout::Grid => LayoutSpec::Grid,
    }
}

/// Builds a `LayoutSpec` for a given layout type, reading max_columns/max_rows
/// from the config's monitor settings for the specified VD and monitor.
fn build_layout_spec(
    config: &AppConfig,
    vd_index: usize,
    monitor_index: usize,
    layout: Layout,
) -> LayoutSpec {
    // Try to get max_columns/max_rows from config for this monitor.
    let ml = config
        .virtual_desktops
        .get(vd_index)
        .and_then(|vd| vd.monitors.get(monitor_index));

    match layout {
        Layout::Monocle => LayoutSpec::Monocle,
        Layout::Columns => LayoutSpec::Columns {
            max_columns: ml.and_then(|m| m.max_columns).unwrap_or(2),
        },
        Layout::Rows => LayoutSpec::Rows {
            max_rows: ml.and_then(|m| m.max_rows).unwrap_or(2),
        },
        Layout::Grid => LayoutSpec::Grid,
    }
}

fn rects_differ(a: &Rect, b: &Rect, tolerance: i32) -> bool {
    (a.left - b.left).abs() > tolerance
        || (a.top - b.top).abs() > tolerance
        || (a.right - b.right).abs() > tolerance
        || (a.bottom - b.bottom).abs() > tolerance
}
