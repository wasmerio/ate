use async_trait::async_trait;
use log::{warn};
use std::{net::{IpAddr, Ipv6Addr}};
use tokio::sync::{Mutex};
use std::{sync::Arc, collections::hash_map::Entry};
use tokio::sync::mpsc;
use fxhash::FxHashMap;
use crate::{pipe::EventPipe};

use super::core::*;
use crate::comms::*;
use crate::accessor::*;
use crate::chain::*;
use crate::error::*;
use crate::conf::*;
use crate::transaction::*;
use super::client::MeshClient;

pub(super) struct MeshRoot {
    cfg: Config,
    lookup: MeshHashTable,
    client: Arc<MeshClient>,
    addrs: Vec<MeshAddress>,
    chains: Mutex<FxHashMap<ChainKey, Arc<ChainAccessor>>>,
}

impl MeshRoot
{
    #[allow(dead_code)]
    pub(super) async fn new(cfg: &Config, listen_addrs: Vec<MeshAddress>) -> Arc<MeshRoot>
    {
        let mut node_cfg = NodeConfig::new()
            .buffer_size(cfg.buffer_size_server);
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

        let ret = Arc::new(
            MeshRoot
            {
                cfg: cfg.clone(),
                addrs: listen_addrs,
                lookup: MeshHashTable::new(cfg),
                client: MeshClient::new(cfg).await,
                chains: Mutex::new(FxHashMap::default()),
            }
        );

        let listen = Node::new(&node_cfg).await;
        tokio::spawn(MeshRoot::inbox(Arc::clone(&ret), listen.inbox));

        ret
    }

    async fn inbox_packet(self: &Arc<MeshRoot>, mut pck: Packet<Message>)
        -> Result<(), CommsError>
    {
        let reply_at_owner = pck.reply_here.take();
        let reply_at = reply_at_owner.as_ref();
        match pck.msg {
            Message::Subscribe(key) =>
            {
                let chain = match self.open(key.clone()).await {
                    Err(ChainCreationError::NoRootFound) => {
                        Packet::reply_at(reply_at, Message::NotThisRoot).await?;
                        return Ok(());
                    }
                    a => a?
                };
                chain.flush().await?;
                
                let multi = chain.multi().await;
                Packet::reply_at(reply_at, Message::StartOfHistory).await?;

                let mut evts = Vec::new();
                for evt in multi.inside.chain.history.iter() {
                    let evt = multi.load(evt).await?;
                    let evt = MessageEvent {
                        meta: evt.raw.meta.clone(),
                        data_hash: evt.raw.data_hash.clone(),
                        data: match evt.raw.data {
                            Some(a) => Some(a.to_vec()),
                            None => None,
                        }
                    };
                    evts.push(evt);

                    if evts.len() > 100 {
                        Packet::reply_at(reply_at, Message::Events {
                            key: key.clone(),
                            commit: None,
                            evts
                        }).await?;
                        evts = Vec::new();
                    }
                }
                if evts.len() > 0 {
                    Packet::reply_at(reply_at, Message::Events {
                        key: key.clone(),
                        commit: None,
                        evts
                    }).await?;
                }
                Packet::reply_at(reply_at, Message::EndOfHistory).await?;
            },
            Message::Events {
                key,
                commit,
                evts
            } => {
                let chain = match self.open(key.clone()).await {
                    Err(ChainCreationError::NoRootFound) => {
                        Packet::reply_at(reply_at, Message::NotThisRoot).await?;
                        return Ok(());
                    },
                    a => a?
                };
                
                let evts = MessageEvent::convert_from(evts);
                let mut single = chain.single().await;                    
                let ret = single.inside.feed_async(evts).await;
                drop(single);

                if let Some(id) = commit {
                    match ret {
                        Ok(()) => Packet::reply_at(reply_at, Message::Confirmed(id.clone())).await?,
                        Err(err) => Packet::reply_at(reply_at, Message::CommitError{
                            id: id.clone(),
                            err: err.to_string(),
                        }).await?
                    };
                }
            },
            _ => { }
        };
        Ok(())
    }

    async fn inbox(self: Arc<MeshRoot>, mut inbox: mpsc::Receiver<Packet<Message>>)
        -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match MeshRoot::inbox_packet(&self, pck).await {
                Ok(_) => { },
                Err(err) => {
                    debug_assert!(false, "mesh-root-err {:?}", err);
                    warn!("mesh-root-err: {}", err.to_string());
                    continue;
                }
            }
        }
        Ok(())
    }

    async fn open_internal<'a>(&'a self, key: &'a ChainKey)
        -> Result<Arc<ChainAccessor>, ChainCreationError>
    {
        let mut chains = self.chains.lock().await;
        let chain = match chains.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) =>
            {
                match self.lookup.lookup(&key) {
                    Some(addr) if self.addrs.contains(&addr) => addr,
                    _ => { return Err(ChainCreationError::NoRootFound); }
                };

                let builder = ChainOfTrustBuilder::new(&self.cfg);
                v.insert(Arc::new(ChainAccessor::new(builder, &key).await?))
            }
        };
        Ok(Arc::clone(chain))
    }
}

#[async_trait]
impl Mesh
for MeshRoot {
    async fn open<'a>(&'a self, key: ChainKey)
        -> Result<Arc<ChainAccessor>, ChainCreationError>
    {
        Ok(
            match self.open_internal(&key).await {
                Err(ChainCreationError::NotThisRoot) => {
                    return Ok(self.client.open(key).await?);
                }
                a => a?,
            }
        )
    }
}

impl EventPipe
for MeshRoot
{
    fn feed(&self, _trans: Transaction) -> Result<(), CommitError>
    {
        Ok(())
    }
}