//! Pure window-to-area assignment and movement rules.

/// Ordered areas for one monitor on one virtual desktop.
///
/// Empty vectors are intentional holes created by manual movement. The final
/// area may contain multiple windows; all earlier areas contain at most one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AreaState<W> {
    areas: Vec<Vec<W>>,
}

impl<W: Copy + Eq> AreaState<W> {
    pub fn new(area_count: usize) -> Self {
        Self {
            areas: vec![Vec::new(); area_count.max(1)],
        }
    }

    pub fn areas(&self) -> &[Vec<W>] {
        &self.areas
    }

    pub fn area_of(&self, window: W) -> Option<usize> {
        self.areas.iter().position(|area| area.contains(&window))
    }

    pub fn contains(&self, window: W) -> bool {
        self.area_of(window).is_some()
    }

    /// Reconciles this state with windows currently present on its monitor.
    ///
    /// Missing windows represent close/minimize/external removal and compact
    /// the layout. Newly discovered windows are inserted at the first area in
    /// discovery order, so the newest discovered window ends up first.
    pub fn sync_present(&mut self, present: &[W]) {
        let had_removed = self
            .areas
            .iter()
            .flatten()
            .any(|window| !present.contains(window));

        if had_removed {
            for area in &mut self.areas {
                area.retain(|window| present.contains(window));
            }
            self.compact();
        }

        for &window in present {
            if !self.contains(window) {
                self.insert_first(window);
            }
        }
    }

    /// Inserts at area zero and shifts every existing area toward the terminal
    /// area. Existing terminal windows stay there.
    pub fn insert_first(&mut self, window: W) {
        if self.contains(window) {
            return;
        }

        if self.areas.len() == 1 {
            self.areas[0].push(window);
            return;
        }

        let mut incoming = vec![window];
        let last = self.areas.len() - 1;
        for index in 0..last {
            std::mem::swap(&mut incoming, &mut self.areas[index]);
        }

        incoming.append(&mut self.areas[last]);
        self.areas[last] = incoming;
    }

    /// Adds a window to the terminal stack.
    pub fn insert_last(&mut self, window: W) {
        if !self.contains(window) {
            self.areas
                .last_mut()
                .expect("at least one area")
                .push(window);
        }
    }

    /// Removes a window while preserving the source hole.
    pub fn remove_preserve(&mut self, window: W) -> bool {
        for area in &mut self.areas {
            if let Some(index) = area.iter().position(|candidate| *candidate == window) {
                area.remove(index);
                return true;
            }
        }
        false
    }

    /// Moves a window to another area on the same monitor.
    ///
    /// An occupied destination swaps its first window back into the source
    /// area. Other windows already in the terminal stack remain there.
    pub fn move_within(&mut self, window: W, target: usize) -> bool {
        let Some(source) = self.area_of(window) else {
            return false;
        };
        if target >= self.areas.len() || target == source {
            return false;
        }

        self.remove_preserve(window);
        let last = self.areas.len() - 1;
        if target == last {
            if self.areas[target].is_empty() {
                self.areas[target].push(window);
            } else {
                let displaced = self.areas[target].remove(0);
                self.areas[target].insert(0, window);
                self.areas[source].push(displaced);
            }
            return true;
        }

        let displaced = std::mem::take(&mut self.areas[target]);
        self.areas[target].push(window);
        self.areas[source].extend(displaced);
        true
    }

    /// Changes area count after a layout switch and redistributes in visual
    /// order, with all overflow in the terminal area.
    pub fn resize_and_compact(&mut self, area_count: usize) {
        let area_count = area_count.max(1);
        if area_count == self.areas.len() {
            return;
        }

        self.redistribute(area_count);
    }

    /// Redistributes all windows after changing layout, even when both layouts
    /// have the same number of areas.
    pub fn redistribute(&mut self, area_count: usize) {
        let area_count = area_count.max(1);
        let windows = self.take_windows();
        self.areas = vec![Vec::new(); area_count];
        self.distribute(windows);
    }

    fn compact(&mut self) {
        let windows = self.take_windows();
        self.distribute(windows);
    }

    fn take_windows(&mut self) -> Vec<W> {
        self.areas.iter_mut().flat_map(std::mem::take).collect()
    }

    fn distribute(&mut self, windows: Vec<W>) {
        let last = self.areas.len() - 1;
        for (index, window) in windows.into_iter().enumerate() {
            self.areas[index.min(last)].push(window);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveDir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementLayout {
    Monocle,
    Columns,
    Rows,
    Grid { columns: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveTarget {
    None,
    Area(usize),
    PreviousMonitorLast,
    NextMonitorFirst,
}

/// Resolves movement without touching platform state.
pub fn resolve_move(
    layout: MovementLayout,
    area: usize,
    area_count: usize,
    direction: MoveDir,
) -> MoveTarget {
    let area_count = area_count.max(1);
    let last = area_count - 1;
    let area = area.min(last);

    match layout {
        MovementLayout::Monocle => match direction {
            MoveDir::Left => MoveTarget::PreviousMonitorLast,
            MoveDir::Right => MoveTarget::NextMonitorFirst,
            MoveDir::Up | MoveDir::Down => MoveTarget::None,
        },
        MovementLayout::Columns => match direction {
            MoveDir::Left if area > 0 => MoveTarget::Area(area - 1),
            MoveDir::Left => MoveTarget::PreviousMonitorLast,
            MoveDir::Right if area < last => MoveTarget::Area(area + 1),
            MoveDir::Right => MoveTarget::NextMonitorFirst,
            MoveDir::Up | MoveDir::Down => MoveTarget::None,
        },
        MovementLayout::Rows => match direction {
            MoveDir::Left => MoveTarget::PreviousMonitorLast,
            MoveDir::Right => MoveTarget::NextMonitorFirst,
            MoveDir::Up if area > 0 => MoveTarget::Area(area - 1),
            MoveDir::Up => MoveTarget::None,
            MoveDir::Down if area < last => MoveTarget::Area(area + 1),
            MoveDir::Down => MoveTarget::None,
        },
        MovementLayout::Grid { columns } => {
            let columns = columns.max(1);
            let row = area / columns;
            let column = area % columns;
            match direction {
                MoveDir::Left if column > 0 => MoveTarget::Area(area - 1),
                MoveDir::Left => MoveTarget::PreviousMonitorLast,
                MoveDir::Right if column + 1 < columns && area + 1 < area_count => {
                    MoveTarget::Area(area + 1)
                }
                MoveDir::Right => MoveTarget::NextMonitorFirst,
                MoveDir::Up if row > 0 => MoveTarget::Area(area - columns),
                MoveDir::Up => MoveTarget::None,
                MoveDir::Down if area + columns < area_count => MoveTarget::Area(area + columns),
                MoveDir::Down => MoveTarget::None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_windows_shift_every_area_and_stack_overflow() {
        let mut state = AreaState::new(3);
        state.sync_present(&[1, 2, 3, 4]);

        assert_eq!(state.areas(), &[vec![4], vec![3], vec![2, 1]]);
    }

    #[test]
    fn new_window_shifts_an_intentional_hole() {
        let mut state = AreaState::new(4);
        state.insert_first(1);
        assert!(state.move_within(1, 1));

        state.sync_present(&[1, 2]);

        assert_eq!(state.areas(), &[vec![2], vec![], vec![1], vec![]]);
    }

    #[test]
    fn removal_compacts_remaining_windows() {
        let mut state = AreaState::new(3);
        state.sync_present(&[1, 2, 3, 4]);

        state.sync_present(&[1, 2, 4]);

        assert_eq!(state.areas(), &[vec![4], vec![2], vec![1]]);
    }

    #[test]
    fn move_to_empty_area_preserves_source_hole() {
        let mut state = AreaState::new(3);
        state.insert_first(1);

        assert!(state.move_within(1, 1));
        assert_eq!(state.areas(), &[vec![], vec![1], vec![]]);
    }

    #[test]
    fn move_swaps_non_terminal_occupant() {
        let mut state = AreaState::new(3);
        state.sync_present(&[1, 2]);

        assert!(state.move_within(2, 1));
        assert_eq!(state.areas(), &[vec![1], vec![2], vec![]]);
    }

    #[test]
    fn move_into_occupied_terminal_area_swaps_first_window() {
        let mut state = AreaState::new(3);
        state.sync_present(&[1, 2, 3, 4]);

        assert!(state.move_within(4, 2));
        assert_eq!(state.areas(), &[vec![2], vec![3], vec![4, 1]]);
    }

    #[test]
    fn move_into_empty_terminal_area_preserves_source_hole() {
        let mut state = AreaState::new(3);
        state.insert_first(1);

        assert!(state.move_within(1, 2));
        assert_eq!(state.areas(), &[vec![], vec![], vec![1]]);
    }

    #[test]
    fn move_out_of_terminal_swaps_occupant_back_into_stack() {
        let mut state = AreaState::new(3);
        state.sync_present(&[1, 2, 3, 4]);

        assert!(state.move_within(2, 1));
        assert_eq!(state.areas(), &[vec![4], vec![2], vec![1, 3]]);
    }

    #[test]
    fn resize_redistributes_and_retains_all_windows() {
        let mut state = AreaState::new(4);
        state.sync_present(&[1, 2, 3, 4, 5]);

        state.resize_and_compact(2);

        assert_eq!(state.areas(), &[vec![5], vec![4, 3, 2, 1]]);
    }

    #[test]
    fn first_area_monitor_arrival_shifts_only_destination() {
        let mut source = AreaState::new(3);
        source.sync_present(&[10]);
        let mut destination = AreaState::new(3);
        destination.sync_present(&[1, 2, 3, 4]);

        assert!(source.remove_preserve(10));
        destination.insert_first(10);

        assert!(source.areas().iter().all(Vec::is_empty));
        assert_eq!(destination.areas(), &[vec![10], vec![4], vec![3, 2, 1]]);
    }

    #[test]
    fn last_area_monitor_arrival_joins_existing_stack() {
        let mut destination = AreaState::new(3);
        destination.sync_present(&[1, 2, 3, 4]);

        destination.insert_last(10);

        assert_eq!(destination.areas(), &[vec![4], vec![3], vec![2, 1, 10]]);
    }

    #[test]
    fn monocle_arrivals_always_join_single_stack() {
        let mut state = AreaState::new(1);
        state.sync_present(&[1, 2]);
        state.insert_first(3);
        state.insert_last(4);

        assert_eq!(state.areas(), &[vec![1, 2, 3, 4]]);
    }

    #[test]
    fn independent_desktop_states_retain_manual_positions() {
        use std::collections::HashMap;

        let mut states = HashMap::new();
        let mut first_desktop = AreaState::new(3);
        first_desktop.insert_first(1);
        assert!(first_desktop.move_within(1, 2));
        states.insert((0, 0), first_desktop);

        let mut second_desktop = AreaState::new(3);
        second_desktop.sync_present(&[2, 3]);
        states.insert((1, 0), second_desktop);

        assert_eq!(
            states.get(&(0, 0)).expect("first desktop").areas(),
            &[vec![], vec![], vec![1]]
        );
        assert_eq!(
            states.get(&(1, 0)).expect("second desktop").areas(),
            &[vec![3], vec![2], vec![]]
        );
    }

    #[test]
    fn monocle_only_moves_between_monitors_horizontally() {
        assert_eq!(
            resolve_move(MovementLayout::Monocle, 0, 1, MoveDir::Left),
            MoveTarget::PreviousMonitorLast
        );
        assert_eq!(
            resolve_move(MovementLayout::Monocle, 0, 1, MoveDir::Right),
            MoveTarget::NextMonitorFirst
        );
        assert_eq!(
            resolve_move(MovementLayout::Monocle, 0, 1, MoveDir::Up),
            MoveTarget::None
        );
        assert_eq!(
            resolve_move(MovementLayout::Monocle, 0, 1, MoveDir::Down),
            MoveTarget::None
        );
    }

    #[test]
    fn columns_move_horizontally_and_cross_at_edges() {
        assert_eq!(
            resolve_move(MovementLayout::Columns, 1, 3, MoveDir::Left),
            MoveTarget::Area(0)
        );
        assert_eq!(
            resolve_move(MovementLayout::Columns, 1, 3, MoveDir::Right),
            MoveTarget::Area(2)
        );
        assert_eq!(
            resolve_move(MovementLayout::Columns, 0, 3, MoveDir::Left),
            MoveTarget::PreviousMonitorLast
        );
        assert_eq!(
            resolve_move(MovementLayout::Columns, 2, 3, MoveDir::Right),
            MoveTarget::NextMonitorFirst
        );
        assert_eq!(
            resolve_move(MovementLayout::Columns, 1, 3, MoveDir::Down),
            MoveTarget::None
        );
    }

    #[test]
    fn rows_move_vertically_and_cross_monitors_horizontally() {
        assert_eq!(
            resolve_move(MovementLayout::Rows, 1, 3, MoveDir::Up),
            MoveTarget::Area(0)
        );
        assert_eq!(
            resolve_move(MovementLayout::Rows, 1, 3, MoveDir::Down),
            MoveTarget::Area(2)
        );
        assert_eq!(
            resolve_move(MovementLayout::Rows, 1, 3, MoveDir::Left),
            MoveTarget::PreviousMonitorLast
        );
        assert_eq!(
            resolve_move(MovementLayout::Rows, 1, 3, MoveDir::Right),
            MoveTarget::NextMonitorFirst
        );
        assert_eq!(
            resolve_move(MovementLayout::Rows, 0, 3, MoveDir::Up),
            MoveTarget::None
        );
    }

    #[test]
    fn grid_moves_in_two_dimensions_and_only_crosses_horizontally() {
        let grid = MovementLayout::Grid { columns: 3 };
        assert_eq!(resolve_move(grid, 4, 6, MoveDir::Left), MoveTarget::Area(3));
        assert_eq!(
            resolve_move(grid, 4, 6, MoveDir::Right),
            MoveTarget::Area(5)
        );
        assert_eq!(resolve_move(grid, 4, 6, MoveDir::Up), MoveTarget::Area(1));
        assert_eq!(resolve_move(grid, 1, 6, MoveDir::Down), MoveTarget::Area(4));
        assert_eq!(
            resolve_move(grid, 3, 6, MoveDir::Left),
            MoveTarget::PreviousMonitorLast
        );
        assert_eq!(
            resolve_move(grid, 5, 6, MoveDir::Right),
            MoveTarget::NextMonitorFirst
        );
        assert_eq!(resolve_move(grid, 1, 6, MoveDir::Up), MoveTarget::None);
        assert_eq!(resolve_move(grid, 4, 6, MoveDir::Down), MoveTarget::None);
    }
}
