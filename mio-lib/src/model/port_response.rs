use std::net::SocketAddr;
use std::net::IpAddr;
use std::fmt;
use serde::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PortResponse {
    Nop {
        handle: SocketHandle,
        ty: PortNopType,
    },
    Received {
        handle: SocketHandle,
        data: Vec<u8>
    },
    ReceivedFrom {
        handle: SocketHandle,
        peer_addr: SocketAddr,
        data: Vec<u8>,
    },
    TcpAccepted {
        handle: SocketHandle,
        peer_addr: SocketAddr,
    },
    SocketError {
        handle: SocketHandle,
        error: SocketError
    },
    DhcpDeconfigured {
        handle: SocketHandle,
    },
    DhcpConfigured {
        handle: SocketHandle,
        address: IpCidr,
        router: Option<IpAddr>,
        dns_servers: Vec<IpAddr>,
    },
    CidrTable {
        cidrs: Vec<IpCidr>,
    },
    RouteTable {
        routes: Vec<IpRoute>,
    },
    Inited,
}

impl fmt::Display
for PortResponse
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortResponse::Nop {
                handle,
                ty,
            } => write!(f, "nop(handle={}, ty={:?})", handle, ty),
            PortResponse::Received {
                handle,
                data
            } => write!(f, "received(handle={},len={})", handle, data.len()),
            PortResponse::ReceivedFrom {
                handle,
                data,
                peer_addr
            } => write!(f, "received_from(handle={},len={},peer_addr={})", handle, data.len(), peer_addr),
            PortResponse::TcpAccepted {
                handle,
                peer_addr,
            } => write!(f, "tcp_accepted(handle={},peer_addr={})", handle, peer_addr),
            PortResponse::SocketError {
                handle,
                error,
            } => write!(f, "socket-error(handle={},err={})", handle, error),
            PortResponse::DhcpDeconfigured {
                handle,
            } => write!(f, "dhcp-deconfigured(handle={})", handle),
            PortResponse::DhcpConfigured {
                handle,
                address,
                router,
                dns_servers,
            } => {
                write!(f, "dhcp-configured(handle={},address={}", handle, address)?;
                if let Some(router) = router {
                    write!(f, ",router={}", router)?;
                }
                if dns_servers.len() > 0 {
                    write!(f, ",dns-servers=[")?;
                    for dns_server in dns_servers.iter() {
                        write!(f, "{},", dns_server)?;
                    }
                    write!(f, "]")?;
                }
                write!(f, ")")
            },
            PortResponse::CidrTable {
                cidrs,
            } => {
                write!(f, "cidr-table(")?;
                for cidr in cidrs {
                    write!(f, "{}/{}", cidr.ip, cidr.prefix)?;
                }
                write!(f, ")")
            },
            PortResponse::RouteTable {
                routes,
            } => {
                write!(f, "route-table(")?;
                for route in routes {
                    write!(f, "{}/{}->{}", route.cidr.ip, route.cidr.prefix, route.via_router)?;
                }
                write!(f, ")")
            },
            PortResponse::Inited => {
                write!(f, "initialized")
            }
        }
    }
}
