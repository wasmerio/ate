use std::sync::Arc;
use std::io;
use std::ops::Deref;
use std::ops::DerefMut;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use ate_comms::StreamSecurity;
use tokio::sync::Mutex;
use tokio::sync::MutexGuard;
use chrono::DateTime;
use chrono::Utc;
use derivative::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_bus::task::block_on;
use ate_crypto::ChainKey;

use crate::model::IpCidr;
use crate::model::IpRoute;
use crate::model::HardwareAddress;
use crate::model::NetworkToken;
use crate::model::SwitchHello;
use crate::comms::Port as CommsPort;
use super::AsyncIcmpSocket;
use super::AsyncRawSocket;
use super::AsyncTcpListener;
use super::AsyncTcpStream;
use super::AsyncUdpSocket;
use super::IcmpSocket;
use super::RawSocket;
use super::TcpListener;
use super::TcpStream;
use super::UdpSocket;

#[derive(Debug)]
struct State {
    port: CommsPort,
}

#[derive(Debug)]
struct StateGuard<'a>
{
    guard: MutexGuard<'a, Option<State>>
}

impl<'a> Deref
for StateGuard<'a>
{
    type Target = State;

    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap()
    }
}

impl<'a> DerefMut
for StateGuard<'a>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum TokenSource
{
    ByPath(String),
    ByValue(NetworkToken)
}

#[allow(dead_code)]
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Port {
    token: NetworkToken,
    security: StreamSecurity,
    net_url: url::Url,
    state: Arc<Mutex<Option<State>>>
}

impl Port {
    pub fn new(
        token: TokenSource,
        net_url: url::Url,
        security: StreamSecurity
    ) -> io::Result<Self>
    { 
        // Load the access token
        let token = match token {
            TokenSource::ByPath(token_path) => {
                super::load_access_token(token_path)
                    .map_err(|err| {
                        io::Error::new(io::ErrorKind::PermissionDenied, format!("failed to load network access token - {}", err))
                    })?
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::PermissionDenied, format!("failed to find network access token"))
                    })?
            },
            TokenSource::ByValue(a) => a
        };

        Ok(Self {
            token,
            net_url,
            security,
            state: Arc::new(Mutex::new(None))
        })
    }

    async fn get_or_create_state<'a>(&'a self) -> io::Result<StateGuard<'a>>
    {
        // Lock the state guard
        let mut guard = self.state.lock().await;

        // If the port is already set then we are good to go
        if guard.is_some() {
            return Ok(StateGuard {
                guard
            })
        }

        // Create the connection to the servers
        let port = ate_comms::StreamClient::connect(
            self.net_url.clone(),
            self.net_url.path(),
            self.security,
            Some("8.8.8.8".to_string()),
            false)
            .await
            .map_err(|err| {
                io::Error::new(io::ErrorKind::ConnectionReset, format!("failed to connect to mesh network - {}", err))
            })?;
        let (rx, mut tx) = port.split();
        
        // Send the switch hello message
        let hello = SwitchHello {
            chain: self.token.chain.clone(),
            access_token: self.token.access_token.clone(),
            version: crate::model::PORT_COMMAND_VERSION,
        };
        let data = serde_json::to_vec(&hello)?;
        tx.write(&data[..]).await?;

        // Create the port
        let port = CommsPort::new(rx, tx).await?;

        // Set the state
        guard.replace(State {
            port,
        });

        // Return the guard
        Ok(StateGuard {
            guard
        })
    }

    /// Return: First value is the ip address, second is the netmask
    pub async fn dhcp_acquire(&self) -> io::Result<(Ipv4Addr, Ipv4Addr)> {
        let guard = self.get_or_create_state().await?;
        let (ip, netmask) = guard.port
            .dhcp_acquire()
            .await?;
        Ok((ip, netmask))
    }

    /// Return: First value is the ip address, second is the netmask
    pub fn blocking_dhcp_acquire(&self) -> io::Result<(Ipv4Addr, Ipv4Addr)> {
        block_on(self.dhcp_acquire())
    }

    pub fn token(&self) -> &NetworkToken {
        &self.token
    }

    pub fn chain(&self) -> &ChainKey {
        &self.token.chain
    }

    pub async fn add_ip(&self, ip: IpAddr, prefix: u8) -> io::Result<IpCidr> {
        let mut guard = self.get_or_create_state().await?;
        guard.port.add_ip(ip, prefix).await
    }

    pub fn blocking_add_ip(&self, ip: IpAddr, prefix: u8) -> io::Result<IpCidr> {
        block_on(self.add_ip(ip, prefix))
    }

    pub async fn remove_ip(&self, ip: IpAddr) -> io::Result<Option<IpCidr>> {
        let mut guard = self.get_or_create_state().await?;
        guard.port.remove_ip(ip).await
    }

    pub fn blocking_remove_ip(&self, ip: IpAddr) -> io::Result<Option<IpCidr>> {
        block_on(self.remove_ip(ip))
    }

    pub async fn hardware_address(&self) -> io::Result<Option<HardwareAddress>> {
        let guard = self.get_or_create_state().await?;
        Ok(guard.port.hardware_address().await)
    }

    pub fn blocking_hardware_address(&self) -> io::Result<Option<HardwareAddress>> {
        block_on(self.hardware_address())
    }

    pub async fn ips(&self) -> io::Result<Vec<IpCidr>> {
        let guard = self.get_or_create_state().await?;
        Ok(guard.port.ips().await)
    }

    pub fn blocking_ips(&self) -> io::Result<Vec<IpCidr>> {
        block_on(self.ips())
    }

    pub async fn clear_ips(&self) -> io::Result<()> {
        let mut guard = self.get_or_create_state().await?;
        guard.port.clear_ips().await
    }

    pub fn blocking_clear_ips(&self) -> io::Result<()> {
        block_on(self.clear_ips())
    }

    pub async fn add_default_route(&self, gateway: IpAddr) -> io::Result<IpRoute> {
        let mut guard = self.get_or_create_state().await?;
        guard.port.add_default_route(gateway).await
    }

    pub fn blocking_add_default_route(&self, gateway: IpAddr) -> io::Result<IpRoute> {
        block_on(self.add_default_route(gateway))
    }

    pub async fn add_route(&self, cidr: IpCidr, via_router: IpAddr, preferred_until: Option<DateTime<Utc>>, expires_at: Option<DateTime<Utc>>) -> io::Result<IpRoute> {
        let mut guard = self.get_or_create_state().await?;
        guard.port.add_route(cidr, via_router, preferred_until, expires_at).await
    }

    pub fn blocking_add_route(&self, cidr: IpCidr, via_router: IpAddr, preferred_until: Option<DateTime<Utc>>, expires_at: Option<DateTime<Utc>>) -> io::Result<IpRoute> {
        block_on(self.add_route(cidr, via_router, preferred_until, expires_at))
    }

    pub async fn remove_route_by_address(&self, addr: IpAddr) -> io::Result<Option<IpRoute>> {
        let mut guard = self.get_or_create_state().await?;
        guard.port.remove_route_by_address(addr).await
    }

    pub fn blocking_remove_route_by_address(&self, addr: IpAddr) -> io::Result<Option<IpRoute>> {
        block_on(self.remove_route_by_address(addr))
    }

    pub async fn remove_route_by_gateway(&self, gw_ip: IpAddr) -> io::Result<Option<IpRoute>> {
        let mut guard = self.get_or_create_state().await?;
        guard.port.remove_route_by_gateway(gw_ip).await
    }

    pub fn blocking_remove_route_by_gateway(&self, gw_ip: IpAddr) -> io::Result<Option<IpRoute>> {
        block_on(self.remove_route_by_gateway(gw_ip))
    }

    pub async fn route_table(&self) -> io::Result<Vec<IpRoute>> {
        let guard = self.get_or_create_state().await?;
        Ok(guard.port.route_table().await)
    }

    pub fn blocking_route_table(&self) -> io::Result<Vec<IpRoute>> {
        block_on(self.route_table())
    }

    pub async fn clear_route_table(&self) -> io::Result<()> {
        let mut guard = self.get_or_create_state().await?;
        guard.port.clear_route_table().await
    }

    pub fn blocking_clear_route_table(&self) -> io::Result<()> {
        block_on(self.clear_route_table())
    }

    pub async fn addr_ipv4(&self) -> io::Result<Option<Ipv4Addr>> {
        let guard = self.get_or_create_state().await?;
        Ok(guard.port.addr_ipv4().await)
    }

    pub fn blocking_addr_ipv4(&self) -> io::Result<Option<Ipv4Addr>> {
        block_on(self.addr_ipv4())
    }

    pub async fn addr_ipv6(&self) -> io::Result<Vec<Ipv6Addr>> {
        let guard = self.get_or_create_state().await?;
        Ok(guard.port.addr_ipv6().await)
    }

    pub fn blocking_addr_ipv6(&self) -> io::Result<Vec<Ipv6Addr>> {
        block_on(self.addr_ipv6())
    }

    pub fn set_security(&mut self, security: StreamSecurity) {
        self.security = security;
    }

    pub async fn bind_raw(
        &self,
    ) -> io::Result<AsyncRawSocket> {
        let guard = self.get_or_create_state().await?;
        let socket = guard.port.bind_raw()
            .await?;
        Ok(
            AsyncRawSocket::new(socket)
        )
    }

    pub fn blocking_bind_raw(
        &self,
    ) -> io::Result<RawSocket> {
        Ok(RawSocket::new(block_on(self.bind_raw())?))
    }

    pub async fn listen_tcp(
        &self,
        addr: SocketAddr
    ) -> io::Result<AsyncTcpListener> {
        let port = {
            let guard = self.get_or_create_state().await?;
            guard.port.clone()
        };
        Ok(AsyncTcpListener::new(port, addr).await?)
    }

    pub fn blocking_bind_tcp(
        &self,
        addr: SocketAddr
    ) -> io::Result<TcpListener> {
        Ok(TcpListener::new(block_on(self.listen_tcp(addr))?))
    }

    pub async fn bind_udp(
        &self,
        addr: SocketAddr
    ) -> io::Result<AsyncUdpSocket> {
        let (port, socket) = {
            let guard = self.get_or_create_state().await?;
            let port = guard.port.clone();
            let socket = port
                .bind_udp(addr)
                .await?;
            (port, socket)
        };
        Ok(AsyncUdpSocket::new(port, socket, addr))
    }

    pub fn blocking_bind_udp(
        &self,
        addr: SocketAddr
    ) -> io::Result<UdpSocket> {
        Ok(UdpSocket::new(block_on(self.bind_udp(addr))?))
    }

    pub async fn bind_icmp(
        &self,
        addr: IpAddr
    ) -> io::Result<AsyncIcmpSocket> {
        let guard = self.get_or_create_state().await?;
        let port = guard.port.clone();
        let socket = port
            .bind_icmp(addr)
            .await?;
        Ok(AsyncIcmpSocket::new(socket, addr))
    }

    pub fn blocking_bind_icmp(
        &self,
        addr: IpAddr
    ) -> io::Result<IcmpSocket> {
        Ok(IcmpSocket::new(block_on(self.bind_icmp(addr))?))
    }

    pub async fn connect_tcp(
        &self,
        addr: SocketAddr,
        peer: SocketAddr,
    ) -> io::Result<AsyncTcpStream> {
        let guard = self.get_or_create_state().await?;
        let socket = guard.port
            .connect_tcp(addr, peer)
            .await?;
        Ok(AsyncTcpStream::new(socket, addr, peer))
    }

    pub fn blocking_connect_tcp(
        &self,
        addr: SocketAddr,
        peer: SocketAddr
    ) -> io::Result<TcpStream> {
        Ok(TcpStream::new(block_on(self.connect_tcp(addr, peer))?))
    }
}
