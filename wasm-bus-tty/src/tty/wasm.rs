#![allow(dead_code)]
use std::{result::Result, sync::Arc};
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use crate::api;
use std::io;

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
                })
            )
            .await
            .map_err(|err| err.into_io_error())?;

        Ok(Stdin {
            rx_data,
            rx_flush,
            client
        })
    }

    pub async fn stdout() -> Result<Stdout, io::Error> {
        let client = api::TtyClient::new(WAPM_NAME)
            .stdout()
            .await
            .map_err(|err| err.into_io_error())?;

        Ok(Stdout {
            client
        })
    }

    pub async fn stderr() -> Result<Stderr, io::Error> {
        let client = api::TtyClient::new(WAPM_NAME)
            .stderr()
            .await
            .map_err(|err| err.into_io_error())?;

        Ok(Stderr {
            client
        })
    }
}

pub struct Stdin
{
    rx_data: mpsc::Receiver<Vec<u8>>,
    rx_flush: mpsc::Receiver<()>,
    client: Arc<dyn api::Stdin + Send + Sync>,
}

impl Stdin
{
    pub async fn read(&mut self) -> Option<Vec<u8>> {
        self.rx_data.recv().await
    }

    pub async fn wait_for_flush(&mut self) -> Option<()> {
        self.rx_flush.recv().await
    }
}

pub struct Stdout
{
    client: Arc<dyn api::Stdout + Send + Sync>,    
}

impl Stdout
{
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

pub struct Stderr
{
    client: Arc<dyn api::Stderr + Send + Sync>,    
}

impl Stderr
{
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