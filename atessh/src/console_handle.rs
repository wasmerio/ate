use async_trait::async_trait;
use ate_files::prelude::*;
use std::io::{self, Write};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use term_lib::api::ConsoleAbi;
use term_lib::api::ConsoleRect;
use thrussh::server::Handle;
use thrussh::ChannelId;
use thrussh::CryptoVec;
use tokterm::term_lib;

pub struct ConsoleHandle {
    pub rect: Arc<Mutex<ConsoleRect>>,
    pub native_files: Arc<FileAccessor>,
    pub channel: ChannelId,
    pub handle: Handle,
}

#[async_trait]
impl ConsoleAbi for ConsoleHandle {
    /// Writes output to the console
    async fn stdout(&self, data: Vec<u8>) {
        let channel = self.channel;
        let data = CryptoVec::from_slice(&data[..]);
        let mut handle = self.handle.clone();
        let _ = handle.data(channel, data).await;
    }

    /// Writes output to the console
    async fn stderr(&self, data: Vec<u8>) {
        let channel = self.channel;
        let data = CryptoVec::from_slice(&data[..]);
        let mut handle = self.handle.clone();
        let _ = handle.extended_data(channel, 1, data).await;
    }

    /// Writes output to the log
    async fn log(&self, text: String) {
        use raw_tty::GuardMode;
        if let Ok(mut stderr) = io::stderr().guard_mode().unwrap() {
            write!(&mut *stderr, "{}\r\n", text).unwrap();
        }
        stderr.flush().unwrap();
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
