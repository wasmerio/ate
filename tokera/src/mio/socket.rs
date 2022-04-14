use std::io;
use std::sync::Arc;
use std::net::SocketAddr;
use ate::crypto::EncryptKey;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use ate::comms::StreamTx;

use crate::model::PortCommand;
use crate::model::PortNopType;
use crate::model::SocketHandle;
use crate::model::IpProtocol;

use super::evt::*;

pub struct Socket
{
    pub(super) proto: IpProtocol,
    pub(super) handle: SocketHandle,
    pub(super) peer_addr: Option<SocketAddr>,
    pub(super) tx: Arc<Mutex<StreamTx>>,
    pub(super) ek: Option<EncryptKey>,
    pub(super) nop: mpsc::Receiver<PortNopType>,
    pub(super) recv: mpsc::Receiver<EventRecv>,
    pub(super) recv_from: mpsc::Receiver<EventRecvFrom>,
    pub(super) error: mpsc::Receiver<EventError>,
    pub(super) accept: mpsc::Receiver<EventAccept>,
}

impl Socket
{
    pub(super) const HOP_LIMIT: u8 = 64;

    pub async fn send(&self, data: Vec<u8>) -> io::Result<usize> {
        let len = data.len();
        if self.proto.is_connection_oriented() {
            self.tx(PortCommand::Send {
                handle: self.handle,
                data,
            }).await?;
        } else if let Some(peer_addr) = self.peer_addr {
            self.tx(PortCommand::SendTo {
                handle: self.handle,
                data,
                addr: peer_addr,
            }).await?;
        } else {
            return Err(io::Error::from(io::ErrorKind::NotConnected));    
        }
        Ok(len)
    }

    pub async fn send_to(&self, data: Vec<u8>, peer_addr: SocketAddr) -> io::Result<usize> {
        let len = data.len();
        if self.proto.is_connection_oriented() {
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        } else {
            self.tx(PortCommand::SendTo {
                handle: self.handle,
                data,
                addr: peer_addr,
            }).await?;
        }
        Ok(len)
    }

    pub async fn recv(&mut self) -> io::Result<Vec<u8>> {
        if self.proto.is_connection_oriented()
        {
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
        } else if let Some(peer_addr) = self.peer_addr {
            loop {
                tokio::select! {
                    evt = self.recv_from.recv() => {
                        let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                        if evt.peer_addr != peer_addr {
                            continue;
                        }
                        return Ok(evt.data);
                    },
                    evt = self.error.recv() => {
                        let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                        return Err(evt.error.into());
                    }
                }
            }
        } else {
            Err(io::Error::from(io::ErrorKind::NotConnected))
        }
    }

    pub async fn recv_from(&mut self) -> io::Result<(Vec<u8>, SocketAddr)> {
        if let Some(peer_addr) = self.peer_addr {
            tokio::select! {
                evt = self.recv.recv() => {
                    let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                    Ok((evt.data, peer_addr))
                },
                evt = self.recv_from.recv() => {
                    let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                    Ok((evt.data, evt.peer_addr))
                },
                evt = self.error.recv() => {
                    let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                    Err(evt.error.into())
                }
            }
        } else {
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
    }

    pub async fn accept(&mut self) -> io::Result<SocketAddr> {
        tokio::select! {
            evt = self.accept.recv() => {
                let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                self.peer_addr.replace(evt.peer_addr.clone());
                Ok(evt.peer_addr)
            },
            evt = self.error.recv() => {
                let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                Err(evt.error.into())
            }
        }
    }

    pub async fn may_send(&mut self) -> io::Result<bool> {
        self.tx(PortCommand::MaySend {
            handle: self.handle,
        }).await?;
        match self.nop(PortNopType::MaySend).await {
            Ok(()) => Ok(true),
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(false),
            Err(err) => Err(err)
        }
    }

    pub async fn may_receive(&mut self) -> io::Result<bool> {
        self.tx(PortCommand::MayReceive {
            handle: self.handle,
        }).await?;
        match self.nop(PortNopType::MayReceive).await {
            Ok(()) => Ok(true),
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(false),
            Err(err) => Err(err)
        }
    }

    pub(super) async fn nop(&mut self, ty: PortNopType) -> io::Result<()> {
        loop {
            tokio::select! {
                tst = self.nop.recv() => {
                    let tst = tst.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                    if tst != ty {
                        continue;
                    }
                    return Ok(());
                },
                evt = self.error.recv() => {
                    let evt = evt.ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionAborted))?;
                    return Err(evt.error.into());
                }
            }
        }
    }

    pub async fn wait_till_may_send(&mut self) -> io::Result<()> {
        let mut time = 0u64;
        loop {
            time = time * 2;
            time += 1;
            if time > 50 {
                time = 50;
            }

            if self.may_send().await? == true {
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_millis(time)).await;
        }
    }

    pub async fn wait_till_may_receive(&mut self) -> io::Result<()> {
        let mut time = 0u64;
        loop {
            time = time * 2;
            time += 1;
            if time > 50 {
                time = 50;
            }

            if self.may_receive().await? == true {
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_millis(time)).await;
        }
    }

    pub fn peer_addr(&self) -> Option<&SocketAddr> {
        self.peer_addr.as_ref()
    }

    pub fn connect(&mut self, peer_addr: SocketAddr) {
        self.peer_addr.replace(peer_addr);
    }

    pub fn is_connected(&self) -> bool {
        self.proto.is_connection_oriented() ||
        self.peer_addr.is_some()
    }

    pub(super) async fn tx(&self, cmd: PortCommand) -> io::Result<()> {
        let mut tx = self.tx.lock().await;
        let cmd = bincode::serialize(&cmd)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        tx.send(&self.ek, &cmd[..]).await?;
        Ok(())
    }
}