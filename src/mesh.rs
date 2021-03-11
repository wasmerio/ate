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
#[allow(unused_imports)]
use super::chain::*;
use super::conf::*;

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
    async fn new(peer_cfg: NodeConfig<Message>, root_inside: Arc<MeshRootProtected>) -> Arc<MeshPeer> {
        let node: NodeWithReceiver<Message> = Node::new(&peer_cfg).await;
        let peer = Arc::new(MeshPeer {
            chains: RwLock::new(FxHashSet::default()),
            node: node.node,
        });
        
        tokio::spawn(MeshPeer::worker_inbox(peer.clone(), root_inside.clone(), node.inbox));
        peer
    }

    async fn worker_inbox(self: Arc<MeshPeer>, root_inside: Arc<MeshRootProtected>, mut inbox: mpsc::Receiver<Packet<Message>>)
    -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match pck.msg {
                Message::Connected => {
                    self.node.upcast(Message::WhatChains).await?;
                },
                Message::MyChains(chains) => {
                    let mut guard = self.chains.write().await;
                    for chain_key in chains.iter() {
                        guard.insert(chain_key.clone());
                    }
                    drop(guard);

                    let mut guard = root_inside.remote_chains.write().await;
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

#[derive(Default)]
pub struct MeshRootProtected
{
    local_chains: RwLock<FxHashMap<ChainKey, ChainOfTrust>>,
    remote_chains: RwLock<FxHashMap<ChainKey, Arc<MeshPeer>>>,
}

pub struct MeshRoot
{
    #[allow(dead_code)]
    listen: Node<Message>,
    #[allow(dead_code)]
    peers: Vec<Arc<MeshPeer>>,
    #[allow(dead_code)]
    inside: Arc<MeshRootProtected>,
}

impl MeshRoot
{
    pub async fn new(cfg: &Config, listen_addrs: Vec<MeshAddress>) -> MeshRoot {
        let mut node_cfg = NodeConfig::new();
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

        let inside = Arc::new(MeshRootProtected::default());
        
        let listen = Node::new(&node_cfg).await;

        let mut peers = Vec::new();
        for peer_addr in cfg.roots.iter() {
            if listen_addrs.contains(peer_addr) { continue; }

            let peer_cfg = NodeConfig::new()
                .connect_to(peer_addr.ip, peer_addr.port)
                .on_connect(Message::Connected);

            peers.push(MeshPeer::new(peer_cfg, inside.clone()).await);
        }

        tokio::spawn(MeshRoot::root_inbox(listen.inbox, inside.clone()));

        MeshRoot {
            listen: listen.node,
            peers,
            inside,
        }
    }

    async fn root_inbox(mut inbox: mpsc::Receiver<Packet<Message>>, root_inside: Arc<MeshRootProtected>)
    -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match pck.msg {
                Message::WhatChains => {
                    let chains = root_inside.local_chains
                        .read().await
                        .iter()
                        .map(|a| a.1.key.clone())
                        .collect::<Vec<_>>();
                    pck.reply(Message::MyChains(chains)).await?;
                },
                _ => { }
            };            
        }
        Ok(())
    }
}

pub struct Mesh
{
    #[allow(dead_code)]
    builder: ChainOfTrustBuilder,
    #[allow(dead_code)]
    cfg: Config,
    #[allow(dead_code)]
    local_ips: Vec<IpAddr>,
    #[allow(dead_code)]
    root: Option<MeshRoot>,
}

impl Mesh
{
    #[allow(dead_code)]
    pub async fn new(builder: ChainOfTrustBuilder) -> Mesh
    {
        let cfg = builder.cfg.clone();
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
            _ => Some(MeshRoot::new(&cfg, listen_root_addresses).await)
        };

        Mesh {
            builder,
            local_ips,
            cfg,
            root,
        }
    }
}

#[tokio::main]
#[test]
async fn test_mesh()
{
    let mut cfg = Config::default();
    for n in 4000..4002 {
        cfg.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
    }

    let mut mesh_roots = Vec::new();
    for n in 4000..4002 {
        cfg.force_listen = Some(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));

        let builder = ChainOfTrustBuilder::new(&cfg, ConfiguredFor::Balanced);
        let mesh = Mesh::new(builder).await;
        mesh_roots.push(mesh);
    }
    
    cfg.force_listen = None;
    cfg.force_client_only = true;

    let builder = ChainOfTrustBuilder::new(&cfg, ConfiguredFor::Balanced);
    let _mesh_client = Mesh::new(builder).await;

    std::thread::sleep(std::time::Duration::from_secs(100));
}