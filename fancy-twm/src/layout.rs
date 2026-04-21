pub trait TilingLayout {
    fn get_zones(
        &self,
        window_index: usize,
        total_windows: usize,
        columns: usize,
        rows: usize,
    ) -> Vec<usize>;
}

pub struct ColumnsLayout {
    pub split_at: usize,
}

impl TilingLayout for ColumnsLayout {
    fn get_zones(
        &self,
        window_index: usize,
        _total_windows: usize,
        columns: usize,
        rows: usize,
    ) -> Vec<usize> {
        let cols_per_split = columns / self.split_at;

        let split_idx = if window_index < (self.split_at as usize - 1) {
            window_index
        } else {
            self.split_at - 1
        };

        let start_col = split_idx * cols_per_split;
        let end_col = if window_index < (self.split_at as usize - 1) {
            (split_idx + 1) * cols_per_split - 1
        } else {
            columns - 1
        };

        let mut zones = Vec::new();
        for c in start_col..=end_col {
            for r in 0..rows {
                zones.push(get_id(c, r, rows));
            }
        }
        zones
    }
}

pub struct RowsLayout {
    pub split_at: usize,
}

impl TilingLayout for RowsLayout {
    fn get_zones(
        &self,
        window_index: usize,
        _total_windows: usize,
        columns: usize,
        rows: usize,
    ) -> Vec<usize> {
        let rows_per_split = rows / self.split_at;

        let split_idx = if window_index < (self.split_at as usize - 1) {
            window_index
        } else {
            self.split_at - 1
        };

        let start_row = split_idx * rows_per_split;
        let end_row = if window_index < (self.split_at as usize - 1) {
            (split_idx + 1) * rows_per_split - 1
        } else {
            rows - 1
        };

        let mut zones = Vec::new();
        for c in 0..columns {
            for r in start_row..=end_row {
                zones.push(get_id(c, r, rows));
            }
        }
        zones.sort(); // Keep IDs ordered for PowerToys
        zones
    }
}

/// Helper to convert (col, row) coordinates to a Column-Major ID
/// col: 0 to columns-1, row: 0 to rows-1
fn get_id(col: usize, row: usize, max_rows: usize) -> usize {
    (col * max_rows) + (row + 1)
}
