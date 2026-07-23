use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    // Virtual desktop navigation
    MoveToNextVirtualDesktop,
    MoveToPrevVirtualDesktop,
    MoveToVirtualDesktop,
    SwitchToNextVirtualDesktop,
    SwitchToPrevVirtualDesktop,
    SwitchToVirtualDesktop,

    // Tiling
    /// Recompute and apply tiling for the active monitor.
    RetileActiveMonitor,
    /// Recompute and apply tiling for all monitors on the current VD.
    RetileVirtualDesktop,

    // Window movement
    /// Move the focused window one area to the right (crosses monitors).
    MoveWindowRight,
    /// Move the focused window one area to the left (crosses monitors).
    MoveWindowLeft,
    /// Move the focused window one area up (Rows layout only).
    MoveWindowUp,
    /// Move the focused window one area down (Rows layout only).
    MoveWindowDown,

    // Layout cycling
    /// Cycle the layout of the monitor containing the focused window.
    CycleLayout,

    // Layout selection
    /// Set a specific layout for the monitor containing the focused window.
    SetLayout,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeMessage {
    pub command: Command,
    pub args: Option<Vec<String>>,
}
