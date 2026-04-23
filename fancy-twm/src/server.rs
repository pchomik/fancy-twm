use anyhow::Result;
use fancycore::message_types::PipeMessage;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, prelude::*};
use std::{io::Read, sync::mpsc, thread};

const PIPE_NAME: &str = "fancytwm-pipe";

pub struct ServerController {
    receiver: mpsc::Receiver<PipeMessage>,
}

impl ServerController {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let name = PIPE_NAME
                .to_ns_name::<GenericNamespaced>()
                .expect("Failed to create name");
            let listener = ListenerOptions::new()
                .name(name)
                .create_sync()
                .expect("Failed to bind pipe");

            for stream in listener.incoming().filter_map(Result::ok) {
                handle_connection(stream, &sender);
            }
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

fn handle_connection(mut stream: LocalSocketStream, sender: &mpsc::Sender<PipeMessage>) {
    let mut buffer = String::new();
    if let Some(_) = stream.read_to_string(&mut buffer).ok() {
        let buffer = buffer.strip_prefix('\u{feff}').unwrap_or(&buffer);
        if let Ok(msg) = serde_json::from_str::<PipeMessage>(buffer) {
            let _ = sender.send(msg);
        }
    }
}
