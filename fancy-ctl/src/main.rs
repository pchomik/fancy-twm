use crate::ipc::IpcClient;
use anyhow::Result;
use clap::{Parser, Subcommand};
use fancycore::message;

pub mod ipc;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Move active window to next virtual desktop
    MoveToNextVirtualDesktop,
    /// Move active window to previous virtual desktop
    MoveToPrevVirtualDesktop,
    /// Move active window to specified virtual desktop
    MoveToVirtualDesktop {
        /// Virtual desktop index starting from 0
        #[arg(short, long)]
        idx: String,
    },
    /// Switch to next virtual desktop
    SwitchToNextVirtualDesktop,
    /// Switch to previous virtual desktop
    SwitchToPrevVirtualDesktop,
    /// Switch to specified virtual desktop
    SwitchToVirtualDesktop {
        /// Virtual desktop index starting from 0
        #[arg(short, long)]
        idx: String,
    },

    /// Recompute and apply tiling for the active monitor
    RetileMonitor,
    /// Recompute and apply tiling for all monitors on the current virtual desktop
    RetileVd,

    /// Move the focused window one area to the right (crosses monitors)
    MoveRight,
    /// Move the focused window one area to the left (crosses monitors)
    MoveLeft,
    /// Move the focused window one area up (Rows layout only)
    MoveUp,
    /// Move the focused window one area down (Rows layout only)
    MoveDown,

    /// Cycle the layout of the monitor containing the focused window
    CycleLayout,

    /// Set a specific layout for the monitor containing the focused window
    SetLayout {
        /// Layout name: Monocle, Columns, Rows, or Grid (case-insensitive)
        #[arg(short, long)]
        layout: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::MoveToNextVirtualDesktop => {
            let msg = message::PipeMessage {
                command: message::Command::MoveToNextVirtualDesktop,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::MoveToPrevVirtualDesktop => {
            let msg = message::PipeMessage {
                command: message::Command::MoveToPrevVirtualDesktop,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::MoveToVirtualDesktop { idx } => {
            let msg = message::PipeMessage {
                command: message::Command::MoveToVirtualDesktop,
                args: Some(vec![idx.clone()]),
            };
            IpcClient::send(msg)?;
        }
        Commands::SwitchToNextVirtualDesktop => {
            let msg = message::PipeMessage {
                command: message::Command::SwitchToNextVirtualDesktop,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::SwitchToPrevVirtualDesktop => {
            let msg = message::PipeMessage {
                command: message::Command::SwitchToPrevVirtualDesktop,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::SwitchToVirtualDesktop { idx } => {
            let msg = message::PipeMessage {
                command: message::Command::SwitchToVirtualDesktop,
                args: Some(vec![idx.clone()]),
            };
            IpcClient::send(msg)?;
        }

        Commands::RetileMonitor => {
            let msg = message::PipeMessage {
                command: message::Command::RetileActiveMonitor,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::RetileVd => {
            let msg = message::PipeMessage {
                command: message::Command::RetileVirtualDesktop,
                args: None,
            };
            IpcClient::send(msg)?;
        }

        Commands::MoveRight => {
            let msg = message::PipeMessage {
                command: message::Command::MoveWindowRight,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::MoveLeft => {
            let msg = message::PipeMessage {
                command: message::Command::MoveWindowLeft,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::MoveUp => {
            let msg = message::PipeMessage {
                command: message::Command::MoveWindowUp,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::MoveDown => {
            let msg = message::PipeMessage {
                command: message::Command::MoveWindowDown,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::CycleLayout => {
            let msg = message::PipeMessage {
                command: message::Command::CycleLayout,
                args: None,
            };
            IpcClient::send(msg)?;
        }
        Commands::SetLayout { layout } => {
            let msg = message::PipeMessage {
                command: message::Command::SetLayout,
                args: Some(vec![layout.clone()]),
            };
            IpcClient::send(msg)?;
        }
    }

    Ok(())
}
