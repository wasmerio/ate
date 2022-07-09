#![allow(dead_code)]
use crate::api;
use std::io;
use std::{result::Result, sync::Arc};
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

pub const WAPM_NAME: &'static str = "os";
const MAX_MPSC: usize = std::usize::MAX >> 3;

pub struct Tty {
    client: api::TtyClient,
}

impl Tty {
    pub async fn stdin() -> Result<Stdin, std::io::Error> {
        let (tx_data, rx_data) = mpsc::channel(MAX_MPSC);
        let (tx_flush, rx_flush) = mpsc::channel(MAX_MPSC);
        let client = api::TtyClient::new(WAPM_NAME)
            .stdin(
                Box::new(move |data: Vec<u8>| {
                    let _ = tx_data.blocking_send(data);
                }),
                Box::new(move |_| {
                    let _ = tx_flush.blocking_send(());
                }),
            )
            .await
            .map_err(|err| err.into_io_error())?;
        
        Ok(Stdin {
            rx_data,
            rx_flush,
            client,
        })
    }

    pub fn blocking_stdin() -> Result<BlockingStdin, std::io::Error> {
        let (tx_data, rx_data) = mpsc::channel(MAX_MPSC);
        let (tx_flush, rx_flush) = mpsc::channel(MAX_MPSC);
        let client = api::TtyClient::new(WAPM_NAME)
            .blocking_stdin(
                Box::new(move |data: Vec<u8>| {
                    let _ = tx_data.blocking_send(data);
                }),
                Box::new(move |_| {
                    let _ = tx_flush.blocking_send(());
                }),
            )
            .map_err(|err| err.into_io_error())?;
        
        Ok(BlockingStdin {
            rx_data,
            rx_flush,
            client,
        })
    }

    pub async fn stdout() -> Result<Stdout, io::Error> {
        let client = api::TtyClient::new(WAPM_NAME)
            .stdout()
            .await
            .map_err(|err| err.into_io_error())?;

        Ok(Stdout { client })
    }

    pub async fn stderr() -> Result<Stderr, io::Error> {
        let client = api::TtyClient::new(WAPM_NAME)
            .stderr()
            .await
            .map_err(|err| err.into_io_error())?;

        Ok(Stderr { client })
    }

    pub async fn rect() -> Result<TtyRect, io::Error> {
        let rect = api::TtyClient::new(WAPM_NAME)
            .rect()
            .await
            .map_err(|err| err.into_io_error())?;

        Ok(TtyRect {
            rows: rect.rows,
            cols: rect.cols,
        })
    }

    pub fn blocking_rect() -> Result<TtyRect, io::Error> {
        let rect = api::TtyClient::new(WAPM_NAME)
            .blocking_rect()
            .map_err(|err| err.into_io_error())?;

        Ok(TtyRect {
            rows: rect.rows,
            cols: rect.cols,
        })
    }
}

pub struct TtyRect {
    pub cols: u32,
    pub rows: u32,
}

pub struct Stdin {
    rx_data: mpsc::Receiver<Vec<u8>>,
    rx_flush: mpsc::Receiver<()>,
    client: Arc<dyn api::Stdin>,
}

impl Stdin {
    pub async fn read(&mut self) -> Option<Vec<u8>> {
        if let Some(data) = self.rx_data.recv().await {
            if data.len() > 0 {
                return Some(data);
            }
        }
        None
    }

    pub async fn wait_for_flush(&mut self) -> Option<()> {
        self.rx_flush.recv().await
    }
}

pub struct BlockingStdin {
    rx_data: mpsc::Receiver<Vec<u8>>,
    rx_flush: mpsc::Receiver<()>,
    client: Arc<dyn api::Stdin>,
}

impl BlockingStdin {
    pub fn read(&mut self) -> Option<Vec<u8>> {
        if let Some(data) = wasm_bus::task::block_on(self.rx_data.recv()) {
            if data.len() > 0 {
                return Some(data);
            }
        }
        None
    }

    pub fn wait_for_flush(&mut self) -> Option<()> {
        wasm_bus::task::block_on(self.rx_flush.recv())
    }
}

pub struct Stdout {
    client: Arc<dyn api::Stdout>,
}

impl Stdout {
    pub async fn write(&mut self, data: Vec<u8>) -> Result<usize, io::Error> {
        self.client
            .write(data)
            .await
            .map_err(|err| err.into_io_error())
            .map(|ret| match ret {
                api::WriteResult::Success(a) => Ok(a),
                api::WriteResult::Failed(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
            })?
    }

    pub async fn print(&mut self, text: String) -> Result<(), io::Error> {
        let data = text.as_bytes().to_vec();
        self.write(data).await?;
        self.flush().await
    }

    pub async fn println(&mut self, text: String) -> Result<(), io::Error> {
        let data = [text.as_bytes(), "\r\n".as_bytes()].concat();
        self.write(data).await?;
        self.flush().await
    }

    pub async fn flush(&mut self) -> Result<(), io::Error> {
        self.client
            .flush()
            .await
            .map_err(|err| err.into_io_error())?;
        Ok(())
    }
}

pub struct Stderr {
    client: Arc<dyn api::Stderr>,
}

impl Stderr {
    pub async fn write(&mut self, data: Vec<u8>) -> Result<usize, io::Error> {
        self.client
            .write(data)
            .await
            .map_err(|err| err.into_io_error())
            .map(|ret| match ret {
                api::WriteResult::Success(a) => Ok(a),
                api::WriteResult::Failed(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
            })?
    }

    pub async fn flush(&mut self) -> Result<(), io::Error> {
        self.client
            .flush()
            .await
            .map_err(|err| err.into_io_error())?;
        Ok(())
    }
}
