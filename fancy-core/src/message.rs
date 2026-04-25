use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    MoveToNextVirtualDesktop,
    MoveToPrevVirtualDesktop,
    MoveToVirtualDesktop,
    SwitchToNextVirtualDesktop,
    SwitchToPrevVirtualDesktop,
    SwitchToVirtualDesktop,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeMessage {
    pub command: Command,
    pub args: Option<Vec<String>>,
}
