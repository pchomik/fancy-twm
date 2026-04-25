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
    }

    Ok(())
}
