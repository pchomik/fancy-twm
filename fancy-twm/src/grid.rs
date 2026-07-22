//! Grid model — a global `rows × columns` lattice of cells laid over each
//! monitor's work area.
//!
//! The grid only knows about cell indexing and single-cell geometry. Turning a
//! set of cells into a window rectangle is the job of [`crate::position`].

use crate::config::GridConfig;
use crate::platform::Rect;

/// A rectangular lattice of cells.
#[derive(Debug, Clone, Copy)]
pub struct Grid {
    pub rows: usize,
    pub columns: usize,
    /// Gap in pixels between cells and around the work area edges.
    pub gap: i32,
}

impl Grid {
    /// Builds a grid from configuration, clamping to sane minimums.
    pub fn from_config(cfg: &GridConfig) -> Self {
        Self {
            rows: cfg.rows.max(1),
            columns: cfg.columns.max(1),
            gap: cfg.gap.max(0),
        }
    }

    /// Total number of cells in the grid.
    pub fn total_cells(&self) -> usize {
        self.rows * self.columns
    }

    /// Column index of a cell (row-major indexing).
    pub fn cell_col(&self, index: usize) -> usize {
        index % self.columns
    }

    /// Row index of a cell (row-major indexing).
    pub fn cell_row(&self, index: usize) -> usize {
        index / self.columns
    }

    /// Returns a copy of this grid with the gap scaled for the given DPI.
    ///
    /// At 96 DPI (100%) the gap is unchanged. At higher DPI the gap is
    /// proportionally larger so it appears visually consistent across monitors
    /// with different scaling factors.
    pub fn with_scaled_gap(&self, dpi: u32) -> Grid {
        let scaled_gap = (self.gap as f64 * dpi as f64 / 96.0).round() as i32;
        Grid {
            rows: self.rows,
            columns: self.columns,
            gap: scaled_gap,
        }
    }

    /// Pixel geometry `(x, y, width, height)` of a single cell within `work`.
    ///
    /// Gaps are applied around the outer edges and between every pair of
    /// adjacent cells.
    pub fn cell_geometry(&self, col: usize, row: usize, work: &Rect) -> (i32, i32, i32, i32) {
        let g = self.gap;
        let cols = self.columns as i32;
        let rows = self.rows as i32;

        let cell_w = (work.width() - g * (cols + 1)) / cols;
        let cell_h = (work.height() - g * (rows + 1)) / rows;

        let x = work.left + g + col as i32 * (cell_w + g);
        let y = work.top + g + row as i32 * (cell_h + g);

        (x, y, cell_w, cell_h)
    }
}
