use crate::ipc::IpcClient;
use anyhow::Result;
use clap::{Arg, Command, command};
use fancycore::message;

pub mod ipc;

fn main() -> Result<()> {
    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("move-to-next-virtual-desktop"))
        .subcommand(Command::new("move-to-prev-virtual-desktop"))
        .subcommand(
            Command::new("move-to-virtual-desktop")
                .arg(Arg::new("idx").short('i').long("idx").required(true)),
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("move-to-next-virtual-desktop") {
        let msg = message::PipeMessage {
            command: message::Command::MoveToNextVirtualDesktop,
            args: None,
        };
        IpcClient::send(msg)?;
    }

    if let Some(_) = matches.subcommand_matches("move-to-prev-virtual-desktop") {
        let msg = message::PipeMessage {
            command: message::Command::MoveToPrevVirtualDesktop,
            args: None,
        };
        IpcClient::send(msg)?;
    }

    if let Some(cmd) = matches.subcommand_matches("move-to-virtual-desktop") {
        let idx = cmd.get_one::<String>("idx").unwrap();
        let msg = message::PipeMessage {
            command: message::Command::MoveToVirtualDesktop,
            args: Some(vec![idx.clone()]),
        };
        IpcClient::send(msg)?;
    }

    Ok(())
}
