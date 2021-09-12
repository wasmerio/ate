#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::pin::Pin;
use std::io;
use core::task::{Context, Poll};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use std::sync::Arc;
use tokio_rustls::Accept;
use tokio::net::TcpStream;
use std::net::SocketAddr;
use std::future::Future;

use hyper;

use super::stream::*;
use super::acme::*;

pub struct HyperAcceptor
where Self: Send + Sync
{
    pub tcp: TcpListener,
    pub tls: Option<TlsAcceptor>,
    pub accepting: Vec<(Accept<TcpStream>, SocketAddr)>,
}

impl HyperAcceptor
{
    pub fn new(listener: TcpListener, enable_tls: bool) -> HyperAcceptor
    {
        let tls = match enable_tls {
            false => None,
            true => {
                let tls_cfg = {
                    let mut cfg = rustls::ServerConfig::new(rustls::NoClientAuth::new());
                    cfg.cert_resolver = Arc::new(Acme::new());
                    cfg.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);
                    Arc::new(cfg)
                };
                Some(
                    TlsAcceptor::from(tls_cfg)
                )
            }
        };
        HyperAcceptor {
            tcp: listener,
            tls,
            accepting: Vec::new(),
        }
    }
}

impl hyper::server::accept::Accept for HyperAcceptor {
    type Conn = HyperStream;
    type Error = io::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>>
    {
        loop {
            match self.tcp.poll_accept(cx) {
                Poll::Pending => break,
                Poll::Ready(Err(err)) => {
                    return Poll::Ready(Some(Err(err)));
                },
                Poll::Ready(Ok((socket, addr))) => {
                    match &mut self.tls {
                        None => {
                            return Poll::Ready(Some(Ok(HyperStream::PlainTcp((socket, addr)))));                        
                        },
                        Some(tls) => {
                            let accept = tls.accept(socket);
                            self.accepting.push((accept, addr));
                        }
                    };
                },
            };
        }

        let mut ret = None;
        let mut drained = Vec::with_capacity(self.accepting.capacity());
        std::mem::swap(&mut self.accepting, &mut drained);
        for (mut accept, addr) in drained {
            if ret.is_some() {
                self.accepting.push((accept, addr));
                continue;
            }
            let accept_pin = Pin::new(&mut accept);
            match accept_pin.poll(cx) {
                Poll::Pending => {
                    self.accepting.push((accept, addr));
                },
                Poll::Ready(Err(err)) => {
                    warn!("failed to accept connection - {}", err);
                }
                Poll::Ready(Ok(stream)) => {
                    ret = Some((stream, addr));
                }
            }
        }

        if let Some((stream, addr)) = ret {
            let stream = HyperStream::Tls((stream, addr));
            return Poll::Ready(Some(Ok(stream)));
        }

        Poll::Pending
    }
}