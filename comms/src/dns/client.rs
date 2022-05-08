use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use derivative::*;
use tokio::net::TcpStream as TokioTcpStream;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tokio::sync::Mutex;
use {
    trust_dns_client::client::*, trust_dns_client::op::DnsResponse, trust_dns_client::tcp::*,
    trust_dns_proto::iocompat::AsyncIoTokioAsStd, trust_dns_proto::DnssecDnsHandle,
};

pub use {trust_dns_client::error::ClientError, trust_dns_client::rr::*};

#[derive(Derivative)]
#[derivative(Debug)]
pub enum DnsClient {
    Dns {
        dns_server: String,
        #[derivative(Debug = "ignore")]
        #[cfg(feature = "dns")]
        client: Mutex<MemoizeClientHandle<AsyncClient>>,
    },
    DnsSec {
        dns_server: String,
        #[derivative(Debug = "ignore")]
        #[cfg(feature = "dns")]
        client: Mutex<DnssecDnsHandle<MemoizeClientHandle<AsyncClient>>>,
    },
}

impl DnsClient {
    pub async fn connect(dns_server: &str, dns_sec: bool) -> Self {
        debug!("using DNS server: {}", dns_server);
        let addr: SocketAddr = (dns_server.to_string(), 53)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();

        let (stream, sender) = TcpClientStream::<AsyncIoTokioAsStd<TokioTcpStream>>::new(addr);
        let client = AsyncClient::new(stream, sender, None);
        let (client, bg) = client.await.expect("client failed to connect");
        wasm_bus::task::spawn(bg);

        let client = MemoizeClientHandle::new(client);

        match dns_sec {
            false => {
                debug!("configured for DNSSec");
                Self::Dns {
                    dns_server: dns_server.to_string(),
                    client: Mutex::new(client),
                }
            }
            true => {
                debug!("configured for plain DNS");
                Self::DnsSec {
                    dns_server: dns_server.to_string(),
                    client: Mutex::new(DnssecDnsHandle::new(client.clone())),
                }
            }
        }
    }

    pub async fn reconnect(&mut self) {
        let (dns_server, dns_sec) = match self {
            Self::Dns { dns_server, client: _ } => (dns_server.clone(), false),
            Self::DnsSec { dns_server, client: _ } => (dns_server.clone(), true),
        };

        *self = Self::connect(dns_server.as_str(), dns_sec).await;
    }

    pub async fn query(
        &mut self,
        name: Name,
        query_class: DNSClass,
        query_type: RecordType,
    ) -> Result<DnsResponse, ClientError> {
        let ret = {
            match self {
                Self::Dns { client: c, .. } => {
                    let mut c = c.lock().await;
                    c.query(name.clone(), query_class, query_type).await
                }
                Self::DnsSec { client: c, .. } => {
                    let mut c = c.lock().await;
                    c.query(name.clone(), query_class, query_type).await
                }
            }
        };

        match ret {
            Ok(a) => Ok(a),
            Err(_) => {
                self.reconnect().await;

                match self {
                    Self::Dns { client: c, .. } => {
                        let mut c = c.lock().await;
                        c.query(name, query_class, query_type).await
                    }
                    Self::DnsSec { client: c, .. } => {
                        let mut c = c.lock().await;
                        c.query(name, query_class, query_type).await
                    }
                }
            }
        }
    }
}
