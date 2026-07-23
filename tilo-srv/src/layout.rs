//! Tiling layouts — each layout partitions the grid into an ordered list of
//! *areas*, where an area is a set of grid cell indices.
//!
//! Layouts only select cell numbers; the actual pixel geometry is computed by
//! [`crate::position`]. Window-to-area assignment is handled by the tiling
//! engine ([`crate::tiling`]).

use crate::grid::Grid;

/// A tiling layout partitions the grid into ordered areas.
pub trait TilingLayout {
    /// Returns the ordered list of areas. Each area is a set of cell indices
    /// (row-major indexing, consistent with [`Grid`]).
    ///
    /// The number of areas is fixed by the layout configuration. Windows are
    /// assigned to areas by index; windows beyond the last area all share the
    /// final area's rectangle (stacked, topmost in z-order visible).
    fn areas(&self, grid: &Grid) -> Vec<Vec<usize>>;
}

/// Single area covering the whole grid. All windows share the same rectangle;
/// the topmost in z-order is visible (the tiler never hides windows).
#[derive(Debug, Clone, Copy, Default)]
pub struct MonocleLayout;

impl TilingLayout for MonocleLayout {
    fn areas(&self, grid: &Grid) -> Vec<Vec<usize>> {
        vec![(0..grid.total_cells()).collect()]
    }
}

/// Vertical columns. The first `max_columns - 1` areas each occupy a single
/// column; the final area occupies all remaining columns.
#[derive(Debug, Clone, Copy)]
pub struct ColumnsLayout {
    pub max_columns: usize,
}

impl TilingLayout for ColumnsLayout {
    fn areas(&self, grid: &Grid) -> Vec<Vec<usize>> {
        let cols = grid.columns;
        let rows = grid.rows;
        // Number of areas is capped by the available columns.
        let n = self.max_columns.max(1).min(cols);

        let mut areas = Vec::with_capacity(n);
        for area in 0..n {
            let start_col = area;
            let end_col = if area == n - 1 { cols - 1 } else { area };
            let mut cells = Vec::new();
            for c in start_col..=end_col {
                for r in 0..rows {
                    cells.push(r * cols + c);
                }
            }
            areas.push(cells);
        }
        areas
    }
}

/// Horizontal rows. The first `max_rows - 1` areas each occupy a single row;
/// the final area occupies all remaining rows.
#[derive(Debug, Clone, Copy)]
pub struct RowsLayout {
    pub max_rows: usize,
}

impl TilingLayout for RowsLayout {
    fn areas(&self, grid: &Grid) -> Vec<Vec<usize>> {
        let cols = grid.columns;
        let rows = grid.rows;
        let n = self.max_rows.max(1).min(rows);

        let mut areas = Vec::with_capacity(n);
        for area in 0..n {
            let start_row = area;
            let end_row = if area == n - 1 { rows - 1 } else { area };
            let mut cells = Vec::new();
            for r in start_row..=end_row {
                for c in 0..cols {
                    cells.push(r * cols + c);
                }
            }
            areas.push(cells);
        }
        areas
    }
}

/// Grid layout — each window occupies exactly one cell. Cells are enumerated
/// in row-major order (across each row, then the next row). Windows beyond the
/// total cell count all share the last cell's rectangle (stacked).
#[derive(Debug, Clone, Copy, Default)]
pub struct GridLayout;

impl TilingLayout for GridLayout {
    fn areas(&self, grid: &Grid) -> Vec<Vec<usize>> {
        let cols = grid.columns;
        let rows = grid.rows;
        let total = grid.total_cells();

        let mut areas = Vec::with_capacity(total);
        for r in 0..rows {
            for c in 0..cols {
                areas.push(vec![r * cols + c]);
            }
        }
        areas
    }
}
