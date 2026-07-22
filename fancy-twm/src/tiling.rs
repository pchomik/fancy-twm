//! Tiling engine — assigns tracked windows to layout areas on each monitor and
//! applies the computed geometry.
//!
//! # Assignment model
//!
//! Each monitor keeps an ordered list of its windows. A window's area is
//! `min(index, num_areas - 1)`:
//! - index 0 → first area
//! - index 1 → second area
//! - …
//! - index ≥ num_areas-1 → last area (stacked; all share the area's rectangle
//!   and the topmost in z-order is visible — the tiler never hides windows)
//!
//! This naturally satisfies the shift rules:
//! - A new window is inserted at the front (first area); existing windows shift
//!   toward higher areas. Windows already in the last area stay there.
//! - When a window is removed, windows after it shift toward lower areas. Only
//!   a single window from the last area moves up to fill the gap.

use crate::config::{AppConfig, Layout};
use crate::grid::Grid;
use crate::layout::{ColumnsLayout, GridLayout, MonocleLayout, RowsLayout, TilingLayout};
use crate::platform::{self, HWND, MonitorInfo, Rect};
use crate::position;
use std::collections::HashMap;

/// Tiling state for a single monitor.
struct MonitorTiler {
    monitor: MonitorInfo,
    /// Ordered window list with per-window area assignment.
    /// The `usize` is the area index, preserved across VD switches via tags.
    windows: Vec<(HWND, usize)>,
    /// Grid layout only: maps each cell index to the window occupying it.
    /// `None` means the cell is empty. This enables true 2D movement where
    /// each cell is independently addressable (unlike the linear model).
    grid_cells: Vec<Option<HWND>>,
}

impl MonitorTiler {
    fn new(monitor: MonitorInfo) -> Self {
        Self {
            monitor,
            windows: Vec::new(),
            grid_cells: Vec::new(),
        }
    }

    /// Merges the currently-present windows into the maintained order,
    /// using window tags to preserve area assignments across VD switches.
    ///
    /// Tags are read from the `present` list directly (not from `self.windows`)
    /// so that after a VD switch — when `self.windows` still holds the previous
    /// VD's windows — every window's tag is still consulted. This ensures:
    /// - A window tagged `col-2` stays in column 2 after a VD round-trip.
    /// - A lone window tagged `col-2` does not drift to column 1.
    /// - Untagged (new) windows default to area 0.
    ///
    /// After assignment, windows are stable-sorted by area so that the
    /// position-based area mapping in `apply()` produces correct results.
    fn sync(
        &mut self,
        present: &[HWND],
        spec: &LayoutSpec,
        num_areas: usize,
        tags: &HashMap<usize, String>,
    ) {
        // Grid layout uses a dedicated 2D cell model rather than the linear
        // area model used by Monocle/Columns/Rows.
        if matches!(spec, LayoutSpec::Grid) {
            self.sync_grid(present, num_areas, tags);
            return;
        }

        let last_area = num_areas.saturating_sub(1);

        // Build area assignments for ALL present windows from their tags.
        // Reading from `present` (not `self.windows`) is critical: after a VD
        // switch `self.windows` contains the old VD's windows which would all
        // be removed by retain, causing every new-VD window to lose its tag.
        let mut with_areas: Vec<(HWND, usize)> = present
            .iter()
            .map(|&w| {
                let tag = tags.get(&(w.0 as usize)).cloned();
                let area = tag
                    .as_deref()
                    .and_then(|tag| tag_to_area(tag, spec))
                    .map(|a| a.min(last_area))
                    .unwrap_or(0);
                crate::log!(
                    "sync: hwnd={} tag={:?} -> area={} (num_areas={})",
                    w.0 as usize,
                    tag,
                    area,
                    num_areas
                );
                (w, area)
            })
            .collect();

        // Stable sort by area to group windows by their assigned area.
        // Relative order within the same area is preserved (from present order).
        with_areas.sort_by_key(|&(_, area)| area);

        self.windows = with_areas;
        crate::log!(
            "sync: final order = {:?}",
            self.windows
                .iter()
                .map(|(w, a)| (w.0 as usize, *a))
                .collect::<Vec<_>>()
        );
    }

    /// Grid-layout sync: assigns each present window to a cell, preserving
    /// cell assignments via `grid-N` tags. Untagged (new) windows fill the
    /// first empty cell; if all cells are taken they stack in the last cell.
    fn sync_grid(&mut self, present: &[HWND], num_cells: usize, tags: &HashMap<usize, String>) {
        let num_cells = num_cells.max(1);
        let mut cells: Vec<Option<HWND>> = vec![None; num_cells];

        // First pass: place windows that have a valid, free grid tag.
        let mut unplaced: Vec<HWND> = Vec::new();
        for &w in present {
            let cell = tags
                .get(&(w.0 as usize))
                .and_then(|tag| tag.strip_prefix("grid-"))
                .and_then(|n| n.parse::<usize>().ok())
                .and_then(|n| n.checked_sub(1))
                .filter(|&idx| idx < num_cells && cells[idx].is_none());
            match cell {
                Some(idx) => cells[idx] = Some(w),
                None => unplaced.push(w),
            }
        }

        // Second pass: fill empty cells with untagged/displaced windows.
        for w in unplaced {
            if let Some(slot) = cells.iter_mut().find(|c| c.is_none()) {
                *slot = Some(w);
            } else {
                // No empty cell — stack in the last cell.
                cells[num_cells - 1] = Some(w);
            }
        }

        self.grid_cells = cells;
        // Keep `windows` populated for cross-monitor moves and bookkeeping.
        self.windows = self
            .grid_cells
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.map(|w| (w, i)))
            .collect();
    }
}

/// Orchestrates tiling across all monitors for the current virtual desktop.
pub struct TilingEngine {
    grid: Grid,
    tilers: Vec<MonitorTiler>,
    /// Layout selection per monitor index (resolved from config for the active VD).
    layouts: Vec<LayoutSpec>,
    /// Runtime layout overrides keyed by (vd_index, monitor_index).
    /// These persist across VD switches and are only reset on app restart.
    layout_overrides: HashMap<(usize, usize), LayoutSpec>,
    /// Current virtual desktop index (needed for override lookup).
    current_vd: usize,
    /// In-memory window tags keyed by HWND pointer value.
    /// Tags preserve area assignments across VD switches within the same
    /// process lifetime. Using an in-memory map avoids the unsafe pointer
    /// storage that Win32 SetPropW would require.
    window_tags: HashMap<usize, String>,
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
            layouts,
            layout_overrides: HashMap::new(),
            current_vd: vd_index,
            window_tags: HashMap::new(),
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

        for (i, (tiler, present)) in self.tilers.iter_mut().zip(per_monitor.iter()).enumerate() {
            let spec = &self.layouts[i];
            let num_areas = spec.build().areas(&self.grid).len().max(1);
            crate::log!(
                "retile: monitor {} spec={:?} num_areas={} present={:?}",
                i,
                spec,
                num_areas,
                present.iter().map(|w| w.0 as usize).collect::<Vec<_>>()
            );
            tiler.sync(present, spec, num_areas, &self.window_tags);
        }

        self.apply();
    }

    /// Applies the current assignment to the screen.
    ///
    /// Every tracked window is positioned; nothing is ever hidden. Windows
    /// beyond the last area all share the last area's rectangle, stacked on
    /// top of each other — the topmost in z-order is the visible one, and
    /// z-order is left untouched (managed by Windows / the user).
    ///
    /// After positioning, each window's tag is updated to reflect its current
    /// area assignment so that VD switches preserve positions.
    ///
    /// Gaps are scaled per-monitor based on DPI so they appear visually
    /// consistent across monitors with different scaling factors.
    fn apply(&mut self) {
        for (tiler, spec) in self.tilers.iter_mut().zip(self.layouts.iter()) {
            let dpi = platform::get_monitor_dpi(tiler.monitor.handle);
            let grid = self.grid.with_scaled_gap(dpi);
            let layout = spec.build();
            let areas = layout.areas(&grid);
            let num_areas = areas.len().max(1);
            let work = tiler.monitor.work_area;
            let last_area_idx = num_areas - 1;

            if matches!(spec, LayoutSpec::Grid) {
                // Grid: position each window at its assigned cell.
                for (cell_idx, slot) in tiler.grid_cells.iter().enumerate() {
                    let Some(hwnd) = slot else { continue };
                    let area_idx = cell_idx.min(last_area_idx);
                    if let Some(rect) =
                        position::calculate_window_rect(&areas[area_idx], &grid, &work)
                    {
                        platform::set_window_pos(*hwnd, rect);
                    }
                    let tag = area_to_tag(spec, area_idx);
                    self.window_tags.insert(hwnd.0 as usize, tag);
                }
            } else {
                for (i, &(hwnd, stored_area)) in tiler.windows.iter().enumerate() {
                    let area_idx = stored_area.min(last_area_idx);
                    let rect = position::calculate_window_rect(&areas[area_idx], &grid, &work);
                    if let Some(r) = rect {
                        platform::set_window_pos(hwnd, r);
                    }
                    // Update the window's tag to reflect its current area.
                    let tag = area_to_tag(spec, area_idx);
                    crate::log!(
                        "apply: hwnd={} pos={} area={} tag={} rect={:?}",
                        hwnd.0 as usize,
                        i,
                        area_idx,
                        tag,
                        rect
                    );
                    self.window_tags.insert(hwnd.0 as usize, tag);
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
    fn area_of(&self, hwnd: HWND) -> Option<(usize, usize, usize)> {
        let (mi, tiler) = self
            .tilers
            .iter()
            .enumerate()
            .find(|(_, t)| t.windows.iter().any(|(w, _)| *w == hwnd))?;
        let pos = tiler.windows.iter().position(|(w, _)| *w == hwnd)?;
        let area = tiler.windows[pos].1;
        Some((mi, pos, area))
    }

    /// Moves the focused window one area in the given direction, crossing
    /// monitors at the edges. Returns `true` if a re-tile happened.
    pub fn move_focused(&mut self, dir: MoveDir) -> bool {
        let Some(hwnd) = platform::get_foreground_window() else {
            return false;
        };

        // Grid layout uses a dedicated 2D movement model where each cell is
        // independently addressable (including empty cells).
        if let Some(mi) = self.monitor_index_for(hwnd) {
            if matches!(self.layouts[mi], LayoutSpec::Grid) {
                return self.move_focused_grid(hwnd, mi, dir);
            }
        }

        let Some((mi, pos, area)) = self.area_of(hwnd) else {
            return false;
        };

        let spec = &self.layouts[mi];
        let areas = spec.build().areas(&self.grid);
        let num_areas = areas.len().max(1);
        let last = num_areas - 1;

        match dir {
            MoveDir::Left | MoveDir::Up => {
                if area == 0 {
                    // Try to move to the previous monitor's last area.
                    return self.move_to_monitor(hwnd, mi, mi.checked_sub(1), true);
                }
                // Shift toward the front: swap HWNDs, areas stay at positions.
                let tiler = &mut self.tilers[mi];
                if pos > 0 {
                    let tmp = tiler.windows[pos].0;
                    tiler.windows[pos].0 = tiler.windows[pos - 1].0;
                    tiler.windows[pos - 1].0 = tmp;
                } else {
                    // No window to swap with — move into the previous area directly.
                    tiler.windows[pos].1 = area - 1;
                }
            }
            MoveDir::Right | MoveDir::Down => {
                if area >= last {
                    // Try to move to the next monitor's first area.
                    return self.move_to_monitor(hwnd, mi, Some(mi + 1), false);
                }
                // Shift toward the back: swap HWNDs, areas stay at positions.
                let tiler = &mut self.tilers[mi];
                if pos + 1 < tiler.windows.len() {
                    let tmp = tiler.windows[pos].0;
                    tiler.windows[pos].0 = tiler.windows[pos + 1].0;
                    tiler.windows[pos + 1].0 = tmp;
                } else {
                    // No window to swap with — move into the next area directly.
                    tiler.windows[pos].1 = area + 1;
                }
            }
        }

        self.apply();
        true
    }

    /// Grid-layout movement: moves the focused window one cell in `dir`,
    /// treating the grid as a true 2D surface. The window can move into empty
    /// cells (swapping with whatever occupies the target, including nothing).
    /// Moving off an edge crosses to the adjacent monitor.
    fn move_focused_grid(&mut self, hwnd: HWND, mi: usize, dir: MoveDir) -> bool {
        let cols = self.grid.columns.max(1);
        let rows = self.grid.rows.max(1);

        // Locate the window's current cell.
        let Some(idx) = self.tilers[mi]
            .grid_cells
            .iter()
            .position(|c| *c == Some(hwnd))
        else {
            return false;
        };

        let row = idx / cols;
        let col = idx % cols;

        // Compute the target cell within this monitor, if any.
        let target = match dir {
            MoveDir::Left => col.checked_sub(1).map(|c| row * cols + c),
            MoveDir::Right => (col + 1 < cols).then_some(row * cols + col + 1),
            MoveDir::Up => row.checked_sub(1).map(|r| r * cols + col),
            MoveDir::Down => (row + 1 < rows).then_some((row + 1) * cols + col),
        };

        match target {
            Some(t) => {
                // Swap within the grid; the target cell may be empty.
                let tiler = &mut self.tilers[mi];
                tiler.grid_cells.swap(idx, t);
                tiler.windows = tiler
                    .grid_cells
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.map(|w| (w, i)))
                    .collect();
                self.apply();
                true
            }
            None => {
                // Off the edge — cross to the adjacent monitor.
                let to_last = matches!(dir, MoveDir::Left | MoveDir::Up);
                let to = if to_last {
                    mi.checked_sub(1)
                } else {
                    Some(mi + 1)
                };
                self.move_grid_to_monitor(hwnd, mi, to, to_last)
            }
        }
    }

    /// Moves a window out of a grid monitor into a target monitor (which may
    /// itself be a grid or a linear layout). `to_last` selects the last cell/
    /// area (for Left/Up) or the first (for Right/Down) on the target.
    fn move_grid_to_monitor(
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

        // Remove from the source grid.
        {
            let src = &mut self.tilers[from];
            for slot in src.grid_cells.iter_mut() {
                if *slot == Some(hwnd) {
                    *slot = None;
                }
            }
            src.windows.retain(|(w, _)| *w != hwnd);
        }

        // Insert into the target monitor.
        if matches!(self.layouts[to], LayoutSpec::Grid) {
            let dst = &mut self.tilers[to];
            let num_cells = dst.grid_cells.len();
            if num_cells > 0 {
                // Prefer the first/last empty cell; fall back to the edge cell.
                let preferred = if to_last {
                    dst.grid_cells
                        .iter()
                        .rposition(|c| c.is_none())
                        .unwrap_or(num_cells - 1)
                } else {
                    dst.grid_cells.iter().position(|c| c.is_none()).unwrap_or(0)
                };
                dst.grid_cells[preferred] = Some(hwnd);
                dst.windows = dst
                    .grid_cells
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.map(|w| (w, i)))
                    .collect();
            } else {
                dst.windows.push((hwnd, 0));
            }
        } else {
            let num_areas = self.layouts[to].build().areas(&self.grid).len().max(1);
            if to_last {
                self.tilers[to].windows.push((hwnd, num_areas - 1));
            } else {
                self.tilers[to].windows.insert(0, (hwnd, 0));
            }
        }

        self.apply();
        true
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

        // Remove from source monitor.
        self.tilers[from].windows.retain(|(w, _)| *w != hwnd);

        // Insert into target. If the target is a Grid layout, place into a
        // cell; otherwise use the linear first/last area model.
        if matches!(self.layouts[to], LayoutSpec::Grid) {
            let dst = &mut self.tilers[to];
            let num_cells = dst.grid_cells.len();
            if num_cells > 0 {
                let preferred = if to_last {
                    dst.grid_cells
                        .iter()
                        .rposition(|c| c.is_none())
                        .unwrap_or(num_cells - 1)
                } else {
                    dst.grid_cells.iter().position(|c| c.is_none()).unwrap_or(0)
                };
                dst.grid_cells[preferred] = Some(hwnd);
                dst.windows = dst
                    .grid_cells
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.map(|w| (w, i)))
                    .collect();
            } else {
                dst.windows.push((hwnd, 0));
            }
        } else {
            let num_areas = self.layouts[to].build().areas(&self.grid).len().max(1);
            if to_last {
                self.tilers[to].windows.push((hwnd, num_areas - 1));
            } else {
                self.tilers[to].windows.insert(0, (hwnd, 0));
            }
        }

        // apply() positions the window at its final rect on the target
        // monitor in a single SetWindowPos call.
        self.apply();
        true
    }

    /// Ensures `grid_cells` is populated for a monitor that just switched to
    /// Grid layout. Distributes the existing window list across cells in order.
    fn ensure_grid_cells(&mut self, mi: usize) {
        let num_cells = self.grid.total_cells();
        let tiler = &mut self.tilers[mi];
        if tiler.grid_cells.len() != num_cells {
            let mut cells: Vec<Option<HWND>> = vec![None; num_cells];
            for (i, &(w, _)) in tiler.windows.iter().enumerate() {
                let idx = i.min(num_cells - 1);
                cells[idx] = Some(w);
            }
            tiler.grid_cells = cells;
        }
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
        let Some(mi) = self.monitor_index_for(hwnd) else {
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
        if matches!(self.layouts[mi], LayoutSpec::Grid) {
            self.ensure_grid_cells(mi);
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
        let Some(mi) = self.monitor_index_for(hwnd) else {
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
        if matches!(self.layouts[mi], LayoutSpec::Grid) {
            self.ensure_grid_cells(mi);
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

    /// Verifies window positions and corrects any that drifted beyond
    /// `tolerance`. Returns the number of windows corrected.
    pub fn verify_positions(&self, tolerance: i32) -> usize {
        let mut corrected = 0;

        for (tiler, spec) in self.tilers.iter().zip(self.layouts.iter()) {
            let dpi = platform::get_monitor_dpi(tiler.monitor.handle);
            let grid = self.grid.with_scaled_gap(dpi);
            let layout = spec.build();
            let areas = layout.areas(&grid);
            let num_areas = areas.len().max(1);
            let work = tiler.monitor.work_area;
            let last = num_areas - 1;

            if matches!(spec, LayoutSpec::Grid) {
                // Grid: verify each occupied cell.
                for (cell_idx, slot) in tiler.grid_cells.iter().enumerate() {
                    let Some(hwnd) = slot else { continue };
                    let area_idx = cell_idx.min(last);
                    let Some(expected) =
                        position::calculate_window_rect(&areas[area_idx], &grid, &work)
                    else {
                        continue;
                    };
                    let Some(actual) = platform::get_window_rect(*hwnd) else {
                        continue;
                    };
                    if rects_differ(&actual, &expected, tolerance) {
                        platform::set_window_pos(*hwnd, expected);
                        corrected += 1;
                    }
                }
            } else {
                for &(hwnd, stored_area) in tiler.windows.iter() {
                    let area_idx = stored_area.min(last);

                    let Some(expected) =
                        position::calculate_window_rect(&areas[area_idx], &grid, &work)
                    else {
                        continue;
                    };

                    let Some(actual) = platform::get_window_rect(hwnd) else {
                        continue;
                    };

                    if rects_differ(&actual, &expected, tolerance) {
                        platform::set_window_pos(hwnd, expected);
                        corrected += 1;
                    }
                }
            }
        }

        corrected
    }
}

/// Direction for window movement commands.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MoveDir {
    Left,
    Right,
    Up,
    Down,
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

/// Converts an area index to a tag string based on the layout type.
///
/// Tag format: `mono` for Monocle, `col-N` for Columns, `row-N` for Rows
/// (1-indexed).
fn area_to_tag(spec: &LayoutSpec, area_index: usize) -> String {
    match spec {
        LayoutSpec::Monocle => "mono".to_string(),
        LayoutSpec::Columns { .. } => format!("col-{}", area_index + 1),
        LayoutSpec::Rows { .. } => format!("row-{}", area_index + 1),
        LayoutSpec::Grid => format!("grid-{}", area_index + 1),
    }
}

/// Converts a tag string to an area index, validating against the layout type.
///
/// Returns `None` if the tag doesn't match the layout type or is malformed.
/// The caller clamps the returned area to the actual number of areas.
fn tag_to_area(tag: &str, spec: &LayoutSpec) -> Option<usize> {
    match spec {
        LayoutSpec::Monocle => {
            if tag == "mono" {
                Some(0)
            } else {
                None
            }
        }
        LayoutSpec::Columns { .. } => {
            let n = tag.strip_prefix("col-")?.parse::<usize>().ok()?;
            if n >= 1 { Some(n - 1) } else { None }
        }
        LayoutSpec::Rows { .. } => {
            let n = tag.strip_prefix("row-")?.parse::<usize>().ok()?;
            if n >= 1 { Some(n - 1) } else { None }
        }
        LayoutSpec::Grid => {
            let n = tag.strip_prefix("grid-")?.parse::<usize>().ok()?;
            if n >= 1 { Some(n - 1) } else { None }
        }
    }
}

fn rects_differ(a: &Rect, b: &Rect, tolerance: i32) -> bool {
    (a.left - b.left).abs() > tolerance
        || (a.top - b.top).abs() > tolerance
        || (a.right - b.right).abs() > tolerance
        || (a.bottom - b.bottom).abs() > tolerance
}
