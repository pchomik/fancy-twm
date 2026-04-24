use anyhow::{Context, Result};
use fancycore::message::PipeMessage;
use interprocess::os::windows::named_pipe::{DuplexPipeStream, pipe_mode};

const PIPE_NAME: &str = r"\\.\pipe\fancytwm-pipe";

pub struct IpcClient {}

impl IpcClient {
    pub fn send(msg: PipeMessage) -> Result<()> {
        let msg = serde_json::to_string(&msg).context("Failed to convert to string")?;
        let conn = DuplexPipeStream::<pipe_mode::Messages>::connect_by_path(PIPE_NAME)
            .context("Failed to connect")?;
        conn.send(msg.as_bytes())
            .context("Failed to send message")?;
        Ok(())
    }
}
