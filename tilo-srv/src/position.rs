//! Position calculation — converts a set of grid cells assigned to a window
//! into a concrete pixel rectangle on a monitor's work area.
//!
//! This is the "something different" referenced in the requirements: layouts
//! only select cell numbers, while this module computes the actual geometry.

use crate::grid::Grid;
use crate::platform::Rect;

/// Computes the pixel rectangle for a window occupying the given `cells`.
///
/// The result is the bounding box that spans all provided cells, including the
/// gaps *between* the spanned cells (so a multi-cell window fills its whole
/// block). Returns `None` if `cells` is empty.
pub fn calculate_window_rect(cells: &[usize], grid: &Grid, work: &Rect) -> Option<Rect> {
    if cells.is_empty() {
        return None;
    }

    let mut min_col = usize::MAX;
    let mut max_col = 0usize;
    let mut min_row = usize::MAX;
    let mut max_row = 0usize;

    for &cell in cells {
        let col = grid.cell_col(cell);
        let row = grid.cell_row(cell);
        min_col = min_col.min(col);
        max_col = max_col.max(col);
        min_row = min_row.min(row);
        max_row = max_row.max(row);
    }

    let (x0, y0, _, _) = grid.cell_geometry(min_col, min_row, work);
    let (x1, y1, w1, h1) = grid.cell_geometry(max_col, max_row, work);

    Some(Rect {
        left: x0,
        top: y0,
        right: x1 + w1,
        bottom: y1 + h1,
    })
}
