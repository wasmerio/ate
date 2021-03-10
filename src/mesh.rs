use serde::{Serialize, Deserialize};
#[allow(unused_imports)]
use std::{net::{IpAddr, Ipv6Addr}, str::FromStr};
use tokio::sync::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use fxhash::FxHashSet;
use fxhash::FxHashMap;

use super::comms::*;
use super::accessor::*;
use super::chain::*;
use super::error::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Noop,
    Connected,
    Disconnected,
    StartOfHistory,
    ProcessEvent {
        meta: Vec<u8>,
        data: Vec<u8>
    },
    EndOfHistory,
    
    /// Asks to confirm all events are up-to-date for transaction keeping purposes
    Confirm(u64),
    Confirmed(u64),

    /// Queries for the list of chains that are running locally to this root
    WhatChains,
    MyChains(Vec<ChainKey>),
}

impl Default
for Message
{
    fn default() -> Message {
        Message::Noop
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeshAddress
{
    pub ip: IpAddr,
    pub port: u16,
}

impl MeshAddress
{
    #[allow(dead_code)]
    pub fn new(ip: IpAddr, port: u16) -> MeshAddress {
        MeshAddress {
            ip: ip,
            port,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct MeshConfig
{
    pub roots: Vec<MeshAddress>,
    pub force_client_only: bool,
    pub force_listen: Option<MeshAddress>,
}

#[allow(dead_code)]
pub struct MeshSession
{
    chain: ChainAccessor,
    comms: Node<Message>
}

#[derive(Debug)]
pub struct MeshPeer
{
    chains: RwLock<FxHashSet<ChainKey>>,
    node: Node<Message>,
}

impl MeshPeer
{
    async fn new(peer_cfg: NodeConfig<Message>, chain_lookup: Arc<RwLock<FxHashMap<ChainKey, Arc<MeshPeer>>>>) -> Arc<MeshPeer> {
        let node: NodeWithReceiver<Message> = Node::new(&peer_cfg).await;
        let peer = Arc::new(MeshPeer {
            chains: RwLock::new(FxHashSet::default()),
            node: node.node,
        });
        
        tokio::spawn(MeshPeer::worker_inbox(peer.clone(), chain_lookup.clone(), node.inbox));
        peer
    }

    async fn worker_inbox(self: Arc<MeshPeer>, chain_lookup: Arc<RwLock<FxHashMap<ChainKey, Arc<MeshPeer>>>>, mut inbox: mpsc::Receiver<Packet<Message>>)
    -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match pck.msg {
                Message::Connected => {
                    self.node.upcast(Message::WhatChains).await?;
                },
                Message::MyChains(chains) => {
                    println!("BLAH!!!");

                    let mut guard = self.chains.write().await;
                    for chain_key in chains.iter() {
                        guard.insert(chain_key.clone());
                    }
                    drop(guard);

                    let mut guard = chain_lookup.write().await;
                    for chain_key in chains.into_iter() {
                        guard.insert(chain_key, self.clone());
                    }
                    drop(guard);
                },
                _ => {}
            };
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct MeshRoot
{
    listen: Node<Message>,
    peers: Vec<Arc<MeshPeer>>,
    chains: Arc<RwLock<FxHashMap<ChainKey, Arc<MeshPeer>>>>,
}

impl MeshRoot
{
    pub async fn new(cfg: &MeshConfig, listen_addrs: Vec<MeshAddress>) -> MeshRoot {
        let mut node_cfg = NodeConfig::new()
            .on_connect(Message::Connected);

        let mut listen_ports = listen_addrs
            .iter()
            .map(|a| a.port)
            .collect::<Vec<_>>();

        listen_ports.sort();
        listen_ports.dedup();
        for port in listen_ports.iter() {
            node_cfg = node_cfg
                .listen_on(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), port.clone());                
        }

        let chains = Arc::new(RwLock::new(FxHashMap::default()));
        
        let listen = Node::new(&node_cfg).await;

        let mut peers = Vec::new();
        for peer_addr in cfg.roots.iter() {
            let peer_cfg = NodeConfig::new()
                .connect_to(peer_addr.ip, peer_addr.port)
                .on_connect(Message::Connected);

            peers.push(MeshPeer::new(peer_cfg, chains.clone()).await);
        }

        tokio::spawn(MeshRoot::root_inbox(listen.inbox));

        MeshRoot {
            listen: listen.node,
            peers,
            chains,
        }
    }

    async fn root_inbox(mut inbox: mpsc::Receiver<Packet<Message>>)
    -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match pck.msg {
                Message::WhatChains => {
                    let chains = Vec::new();
                    pck.reply(Message::MyChains(chains)).await?;
                },
                _ => { }
            };            
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Mesh
{
    cfg: MeshConfig,
    local_ips: Vec<IpAddr>,
    root: Option<MeshRoot>,
}

impl Mesh
{
    #[allow(dead_code)]
    pub async fn new(cfg: &MeshConfig) -> Mesh
    {
        let local_ips = pnet::datalink::interfaces()
            .iter()
            .flat_map(|i| i.ips.iter())
            .map(|i| i.ip())
            .collect::<Vec<_>>();

        let mut listen_root_addresses = Vec::new();
        
        if let Some(addr) = &cfg.force_listen {
            listen_root_addresses.push(addr.clone());
        } else if cfg.force_client_only == false {
            for local_ip in local_ips.iter() {
                for root in cfg.roots.iter() {
                    if root.ip == *local_ip {
                        listen_root_addresses.push(root.clone());
                    }
                }
            }
        }

        let root = match listen_root_addresses.len() {
            0 => None,
            _ => Some(MeshRoot::new(cfg, listen_root_addresses).await)
        };

        Mesh {
            local_ips,
            cfg: cfg.clone(),
            root,
        }
    }
}

#[tokio::main]
#[test]
async fn test_mesh()
{
    let mut cfg = MeshConfig::default();
    for n in 4001..4010 {
        cfg.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
    }

    let mut mesh_roots = Vec::new();
    for n in 4001..4010 {
        cfg.force_listen = Some(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        let mesh = Mesh::new(&cfg).await;
        mesh_roots.push(mesh);
    }
    
    cfg.force_listen = None;
    cfg.force_client_only = true;
    let _mesh_client = Mesh::new(&cfg).await;

    std::thread::sleep(std::time::Duration::from_secs(100));
}