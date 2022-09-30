use async_trait::async_trait;
use std::io::{self, Write};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use wasmer_os::api::ConsoleAbi;
use wasmer_os::api::ConsoleRect;
use thrussh::server::Handle;
use thrussh::ChannelId;
use thrussh::CryptoVec;
use wasmer_term::wasmer_os;

pub struct ConsoleHandle {
    pub rect: Arc<Mutex<ConsoleRect>>,
    pub channel: ChannelId,
    pub handle: Handle,
    pub stdio_lock: Arc<Mutex<()>>,
    pub enable_stderr: bool,
}

#[async_trait]
impl ConsoleAbi
for ConsoleHandle
{
    /// Writes output to the SSH pipe
    async fn stdout(&self, data: Vec<u8>) {
        let channel = self.channel;
        let data = CryptoVec::from_slice(&data[..]);
        let mut handle = self.handle.clone();
        let _ = handle.data(channel, data).await;
    }

    /// Writes output to the SSH pipe
    async fn stderr(&self, data: Vec<u8>) {
        let channel = self.channel;
        let data = CryptoVec::from_slice(&data[..]);
        let mut handle = self.handle.clone();
        if self.enable_stderr {
            let _ = handle.extended_data(channel, 1, data).await;
        } else {
            let _ = handle.data(channel, data).await;
        }
    }

    /// Flushes the data down the SSH pipe
    async fn flush(&self) {
        let channel = self.channel;
        let mut handle = self.handle.clone();
        let _ = handle.flush(channel).await;
    }

    /// Writes output to the log
    async fn log(&self, text: String) {
        use raw_tty::GuardMode;
        let _guard = self.stdio_lock.lock().unwrap();
        if let Ok(mut stderr) = io::stderr().guard_mode() {
            write!(&mut *stderr, "{}\r\n", text).unwrap();
            stderr.flush().unwrap();
        }
    }

    /// Gets the number of columns and rows in the terminal
    async fn console_rect(&self) -> ConsoleRect {
        let rect = self.rect.lock().unwrap();
        rect.deref().clone()
    }

    /// Clears the terminal
    async fn cls(&self) {
        let txt = format!("{}[2J", 27 as char);
        let data = txt.as_bytes().to_vec();
        self.stdout(data).await;
    }

    /// Tell the process to exit (if it can)
    async fn exit(&self) {
        let mut handle = self.handle.clone();
        let _ = handle.close(self.channel).await;
    }
}
