use anyhow::{Context, Result};
use fancycore::message::PipeMessage;
use interprocess::os::windows::named_pipe::{
    DuplexPipeStream, PipeListenerOptions, PipeMode, pipe_mode,
};
use recvmsg::{MsgBuf, sync::RecvMsg};
use std::{path::Path, sync::mpsc, thread};

const PIPE_NAME: &str = r"\\.\pipe\fancytwm-pipe";

pub struct IpcServerController {
    receiver: mpsc::Receiver<PipeMessage>,
}

impl IpcServerController {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let listener = PipeListenerOptions::new()
                .path(Path::new(PIPE_NAME))
                .mode(PipeMode::Messages)
                .create_duplex::<pipe_mode::Messages>()
                .context("Failed to bind pipe")?;

            for stream in listener.incoming().filter_map(Result::ok) {
                handle_connection(stream, &sender);
            }
            Ok::<(), anyhow::Error>(())
        });
        Ok(Self { receiver })
    }

    pub fn read(&self) -> Option<PipeMessage> {
        if let Ok(msg) = self.receiver.try_recv() {
            println!("{:?}", msg);
            Some(msg)
        } else {
            None
        }
    }
}

fn handle_connection(
    mut stream: DuplexPipeStream<pipe_mode::Messages>,
    sender: &mpsc::Sender<PipeMessage>,
) {
    let mut buffer = MsgBuf::new_owned(Vec::new());
    if let Ok(_result) = stream.recv_msg(&mut buffer, None) {
        if let Ok(msg) = serde_json::from_slice::<PipeMessage>(buffer.filled_part()) {
            let _ = sender.send(msg);
        }
    }
}
