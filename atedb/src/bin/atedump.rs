#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use ate::conf::conf_ate;
use serde::{Serialize, Deserialize};
use ate::prelude::*;
use clap::Clap;
use ate::redo::RedoLog;
use ate::loader::Loader;
use ate::loader::LoadData;
use ate::event::*;
use ate::redo::OpenFlags;
use ate::spec::TrustMode;
use ate::trust::ChainHeader;
use async_trait::async_trait;
use ate::utils::LoadProgress;
use tokio::sync::mpsc;
use fxhash::FxHashMap;
use ascii_tree::Tree;
use std::collections::HashMap;

#[derive(Clap)]
#[clap(version = "1.4", author = "John S. <johnathan.sharratt@gmail.com>")]
struct Opts {
    /// Path to the log file to be dumped
    #[clap(index = 1)]
    path: String,
    /// Name of the log file to be opened
    #[clap(index = 2)]
    name: String,
    #[clap(long)]
    no_compact: bool,
    #[clap(long)]
    no_short_names: bool,
}

pub struct DumpLoader
{
    tx: mpsc::Sender<LoadData>,
}

#[async_trait]
impl Loader
for DumpLoader
{
    async fn feed_load_data(&mut self, evt: LoadData) {
        self.tx.send(evt).await.unwrap();
    }
}

#[derive(Default)]
struct EventNode
{
    name: String,
    versions: Vec<String>,
    children: Vec<PrimaryKey>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AteError> {
    let opts: Opts = Opts::parse();
    
    ate::log_init(0, true);

    // Configure
    let flags = OpenFlags {
        read_only: true,
        truncate: false,
        temporal: false,
        integrity: TrustMode::Distributed,
    };
    let header = ChainHeader::default();
    let header_bytes = SerializationFormat::Json.serialize(&header)?;
    let mut cfg_ate = ConfAte::default();
    cfg_ate.log_path = Some(opts.path);
    let key = ChainKey::new(opts.name);

    // Create a progress bar loader
    let mut progress_loader = LoadProgress::default();
    progress_loader.units = pbr::Units::Bytes;
    progress_loader.msg_done = "Reading events from file...".to_string();

    // Build the composite loader
    let (tx, mut rx) = mpsc::channel(u32::MAX as usize);
    let mut loader = Box::new(ate::loader::CompositionLoader::default());
    loader.loaders.push(Box::new(progress_loader));
    loader.loaders.push(Box::new(DumpLoader {
        tx,
    }));
    
    // Load the log file and dump its contents
    RedoLog::open_ext(&cfg_ate, &key, flags, loader, header_bytes).await?;
    
    // Build a tree and dump it to console
    let mut tree_roots: Vec<PrimaryKey> = Vec::new();
    let mut tree_lookup: FxHashMap<PrimaryKey, EventNode> = FxHashMap::default();
    while let Some(evt) = rx.recv().await
    {
        let header = match evt.header.as_header() {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to read metadata - {}", err);
                continue;
            }
        };

        // Build a name for the node
        let name = if let Some(type_name) = header.meta.get_type_name() {
            let mut name = type_name.type_name.clone();
            if opts.no_short_names == false {
                for short in type_name.type_name.split(":") {
                    name = short.to_string();
                }
            }
            if let Some(key) = header.meta.get_data_key() {
                format!("{}({})", name, key)
            } else {
                name
            }
        } else if let Some(tombstone) = header.meta.get_tombstone() {
            if opts.no_compact == false {
                tree_lookup.remove(&tombstone);
                continue;
            }
            format!("tombstone({})", tombstone)
        } else if let Some(pk) = header.meta.get_public_key() {
            format!("public-key({})", pk.hash())
        } else if let Some(sig) = header.meta.get_signature() {
            format!("signature(key={}, for {} records)", sig.public_key_hash, sig.hashes.len())
        } else {
            "unknown".to_string()
        };
        
        // Insert some data into the node
        let key = match header.meta.get_data_key() {
            Some(a) => a,
            None => PrimaryKey::generate(),
        };
        let node = tree_lookup.entry(key).or_insert_with(|| {
            let mut node = EventNode::default();
            node.name = name.clone();
            node
        });

        if opts.no_compact == false {
            node.versions.clear();
        }
        if header.raw.data_size > 0 {
            node.versions.push(format!("data({} bytes)", header.raw.data_size));
        } else if header.meta.get_tombstone().is_some() {
            node.versions.push(name);
        }

        // Put the node in the right place
        match header.meta.get_parent() {
            Some(a) => {
                if let Some(a) = tree_lookup.get_mut(&a.vec.parent_id) {
                    if a.children.iter().any(|a| a.eq(&key)) == false {
                        a.children.push(key);
                    }
                }
            },
            None => {
                if tree_roots.iter().any(|a| a.eq(&key)) == false {
                    tree_roots.push(key);
                }
            }
        }
    }

    // Turn it into ascii-tree
    let mut output = String::new();
    for root in tree_roots.iter() {
        let tree = build_tree(root, &tree_lookup);
        if let Some(tree) = tree {
            ascii_tree::write_tree(&mut output, &tree).unwrap();
        }
    }
    print!("{}", output);
    
    Ok(())
}

fn build_tree(key: &PrimaryKey, tree_lookup: &FxHashMap<PrimaryKey, EventNode>) -> Option<Tree>
{
    if let Some(node) = tree_lookup.get(&key) {
        let mut children = Vec::new();
        if node.versions.len() > 0 {
            let leaf = Tree::Leaf(node.versions.clone());
            children.push(leaf);
        }
        for c in &node.children {
            if let Some(c) = build_tree(c, tree_lookup) {
                children.push(c);
            }
        }
        Some(Tree::Node(node.name.clone(), children))
    } else {
        None
    }
}