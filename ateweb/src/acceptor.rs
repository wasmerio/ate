use super::acme::ACME_TLS_ALPN_NAME;
use core::task::{Context, Poll};
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use hyper;

use super::acme::*;
use super::stream::*;

pub struct HyperAcceptor
where
    Self: Send,
{
    pub tcp: TcpListener,
    pub tls: Option<TlsAcceptor>,
    pub acme: Arc<AcmeResolver>,
    pub accepting:
        Vec<Pin<Box<dyn Future<Output = Result<HyperStream, Box<dyn std::error::Error>>> + Send>>>,
}

impl HyperAcceptor {
    pub fn new(listener: TcpListener, acme: Arc<AcmeResolver>, enable_tls: bool) -> HyperAcceptor {
        let tls = match enable_tls {
            false => None,
            true => {
                let acme = Arc::clone(&acme);
                let tls_cfg = {
                    let mut cfg = rustls::ServerConfig::new(rustls::NoClientAuth::new());
                    cfg.cert_resolver = acme;
                    cfg.set_protocols(&[
                        b"h2".to_vec(),
                        b"http/1.1".to_vec(),
                        b"acme-tls/1".to_vec(),
                    ]);
                    Arc::new(cfg)
                };
                Some(TlsAcceptor::from(tls_cfg))
            }
        };
        HyperAcceptor {
            tcp: listener,
            tls,
            acme,
            accepting: Vec::new(),
        }
    }

    pub async fn accept(
        tls: TlsAcceptor,
        acme: Arc<AcmeResolver>,
        socket: TcpStream,
        addr: SocketAddr,
    ) -> Result<HyperStream, Box<dyn std::error::Error>> {
        // Enter a loop peeking for the hello client message
        let mut peek_size = 128usize;
        while peek_size <= 16384usize {
            peek_size *= 2usize;

            // Keep peeking at the stream until we have a TlsMessage
            let mut buf = vec![0; peek_size];
            let n = socket.peek(&mut buf).await?;
            if n <= 0 {
                continue;
            }

            // Attempt to get a TlsMessage
            let record = match tls_parser::parse_tls_plaintext(&buf[..n]) {
                Ok((_rem, record)) => record,
                Err(tls_parser::Err::Incomplete(_needed)) => {
                    continue;
                }
                Err(e) => {
                    warn!("parse_tls_record_with_header failed: {:?}", e);
                    break;
                }
            };

            // Find the handshake / client hello message
            let msg = record
                .msg
                .iter()
                .filter_map(|a| match a {
                    tls_parser::TlsMessage::Handshake(
                        tls_parser::TlsMessageHandshake::ClientHello(hello),
                    ) => Some(hello),
                    _ => None,
                })
                .next();
            let hello = match msg {
                Some(a) => a,
                None => {
                    continue;
                }
            };

            // Grab all the extensions
            let exts = if let Some(hello_ext) = hello.ext {
                if let Ok((_rem, exts)) = tls_parser::parse_tls_extensions(hello_ext) {
                    exts
                } else {
                    break;
                }
            } else {
                break;
            };

            // If it has an ACME ALPN extension then we dont want to trigger another certificate for it
            // so we instead just attempt to accept the connection
            let mut alpn = false;
            for ext in exts.iter() {
                if let tls_parser::TlsExtension::ALPN(alpn_exts) = ext {
                    for alpn_ext in alpn_exts {
                        if ACME_TLS_ALPN_NAME.eq(*alpn_ext) {
                            alpn = true;
                        }
                    }
                }
            }

            // We are looking for the SNI extension
            let sni = exts
                .iter()
                .filter_map(|a| match a {
                    tls_parser::TlsExtension::SNI(snis) => snis
                        .iter()
                        .filter_map(|a| match a {
                            (tls_parser::SNIType::HostName, sni_bytes) => {
                                Some(String::from_utf8_lossy(sni_bytes))
                            }
                            _ => None,
                        })
                        .next(),
                    _ => None,
                })
                .next();
            let sni = match sni {
                Some(a) => a,
                None => {
                    break;
                }
            };

            // Load the object
            if alpn {
                trace!("alpn challenge for SNI: {}", sni);
                acme.touch_alpn(sni.to_string()).await?;
            } else {
                trace!("connection attempt SNI: {}", sni);
                acme.touch_web(sni.to_string(), chrono::Duration::days(30))
                    .await?;
            }
            break;
        }

        // Its time to now accept the connect (if the preload failed, then so be it, things will still
        // work they will just get a error message on the first request to this web server as it wont
        // have the server ceritifcate loaded yet and will need to be loaded asynchronously)
        let stream = tls.accept(socket).await?;
        Ok(HyperStream::Tls((stream, addr)))
    }
}

impl hyper::server::accept::Accept for HyperAcceptor {
    type Conn = HyperStream;
    type Error = io::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        loop {
            match self.tcp.poll_accept(cx) {
                Poll::Pending => break,
                Poll::Ready(Err(err)) => {
                    return Poll::Ready(Some(Err(err)));
                }
                Poll::Ready(Ok((socket, addr))) => {
                    // For HTTP streams there is nothing more to do
                    let tls = match &self.tls {
                        None => {
                            return Poll::Ready(Some(Ok(HyperStream::PlainTcp((socket, addr)))));
                        }
                        Some(tls) => tls.clone(),
                    };

                    // Otherwise its time to accept the TLS connection
                    let acme = self.acme.clone();
                    let accept = HyperAcceptor::accept(tls, acme, socket, addr);
                    self.accepting.push(Box::pin(accept));
                }
            };
        }

        let mut ret = None;
        let drained = {
            let mut drained = Vec::with_capacity(self.accepting.capacity());
            std::mem::swap(self.accepting.as_mut(), &mut drained);
            drained
        };
        for mut accept in drained {
            if ret.is_some() {
                self.accepting.push(accept);
                continue;
            }
            match accept.as_mut().poll(cx) {
                Poll::Pending => {
                    self.accepting.push(accept);
                }
                Poll::Ready(Ok(stream)) => {
                    ret = Some(stream);
                }
                Poll::Ready(Err(err)) => {
                    warn!("failed to accept TLS stream - {}", err);
                    continue;
                }
            }
        }

        if let Some(stream) = ret {
            return Poll::Ready(Some(Ok(stream)));
        }

        Poll::Pending
    }
}
