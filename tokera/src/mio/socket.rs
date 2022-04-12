use std::io;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use wasm_bus_ws::ws::SendHalf;

use crate::model::PortCommand;
use crate::model::SocketHandle;

use super::evt::*;

pub struct Socket
{
    pub(super) handle: SocketHandle,
    pub(super) tx: SendHalf,
    pub(super) recv: mpsc::Receiver<EventRecv>,
    pub(super) recv_from: mpsc::Receiver<EventRecvFrom>,
    pub(super) error: mpsc::Receiver<EventError>,
    pub(super) accept: mpsc::Receiver<EventAccept>,
    #[allow(dead_code)]
    pub(super) deconfigure: mpsc::Receiver<EventDhcpDeconfigured>,
    #[allow(dead_code)]
    pub(super) configure: mpsc::Receiver<EventDhcpConfigured>,
}

impl Socket
{
    pub(super) const HOP_LIMIT: u8 = 64;

    pub async fn send(&self, data: Vec<u8> ) -> io::Result<usize> {
        let len = data.len();
        self.tx(PortCommand::Send {
            handle: self.handle,
            data,
        }).await?;
        Ok(len)

    }

    pub async fn recv(&mut self) -> io::Result<Vec<u8>> {
        tokio::select! {
            evt = self.recv.recv() => {
                let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                Ok(evt.data)
            },
            evt = self.error.recv() => {
                let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                Err(evt.error.into())
            }
        }
    }

    pub async fn recv_from(&mut self) -> io::Result<(Vec<u8>, SocketAddr)> {
        tokio::select! {
            evt = self.recv_from.recv() => {
                let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                Ok((evt.data, evt.peer_addr))
            },
            evt = self.error.recv() => {
                let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                Err(evt.error.into())
            }
        }
    }

    pub(super) async fn accept(&mut self) -> io::Result<SocketAddr> {
        tokio::select! {
            evt = self.accept.recv() => {
                let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                Ok(evt.peer_addr)
            },
            evt = self.error.recv() => {
                let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                Err(evt.error.into())
            }
        }
    }

    pub(super) async fn tx(&self, cmd: PortCommand) -> io::Result<()> {
        let cmd = bincode::serialize(&cmd)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        self.tx.send(cmd).await?;
        Ok(())
    }
}