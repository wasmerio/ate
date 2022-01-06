#![allow(unused_imports)]
use ascii_tree::Tree;
use async_trait::async_trait;
use ate::conf::conf_ate;
use ate::event::*;
use ate::loader::LoadData;
use ate::loader::Loader;
use ate::prelude::*;
use ate::redo::OpenFlags;
use ate::redo::RedoLog;
use ate::spec::TrustMode;
use ate::trust::ChainHeader;
use ate::utils::LoadProgress;
use clap::Parser;
use colored::*;
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Parser)]
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

pub struct DumpLoader {
    tx: mpsc::Sender<LoadData>,
}

#[async_trait]
impl Loader for DumpLoader {
    async fn feed_load_data(&mut self, evt: LoadData) {
        self.tx.send(evt).await.unwrap();
    }
}

#[derive(Default)]
struct EventNode {
    name: String,
    versions: Vec<AteHash>,
    children: Vec<PrimaryKey>,
}

#[derive(Default)]
struct EventData {
    data: Option<String>,
    data_hash: Option<AteHash>,
    event_hash: Option<AteHash>,
    sig: Option<AteHash>,
    bad_order: bool,
    bad_pk: bool,
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
    let mut progress_loader = LoadProgress::new(std::io::stderr());
    progress_loader.units = pbr::Units::Bytes;
    progress_loader.msg_done = "Reading events from file...".to_string();

    // Build the composite loader
    let (tx, mut rx) = mpsc::channel(u32::MAX as usize);
    let mut loader = Box::new(ate::loader::CompositionLoader::default());
    loader.loaders.push(Box::new(progress_loader));
    loader.loaders.push(Box::new(DumpLoader { tx }));

    // Load the log file and dump its contents
    RedoLog::open_ext(&cfg_ate, &key, flags, loader, header_bytes).await?;

    // Build a tree and dump it to console
    let mut hash_pk = FxHashSet::default();
    let mut tree_pks = Vec::new();
    let mut tree_sigs = Vec::new();
    let mut tree_roots: Vec<PrimaryKey> = Vec::new();
    let mut tree_event: FxHashMap<AteHash, EventData> = FxHashMap::default();
    let mut tree_lookup: FxHashMap<PrimaryKey, EventNode> = FxHashMap::default();
    while let Some(evt) = rx.recv().await {
        let header = match evt.header.as_header() {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to read metadata - {}", err);
                continue;
            }
        };

        // Get the signature and record it for the different data nodes
        if let Some(pk) = header.meta.get_public_key() {
            hash_pk.insert(pk.hash());
            tree_pks.push(pk.hash());
        }

        // Get the signature and record it for the different data nodes
        if let Some(sig) = header.meta.get_signature() {
            for hash in &sig.hashes {
                tree_event.entry(hash.clone()).or_default().sig = Some(sig.public_key_hash);
            }
            tree_sigs.push(sig.clone());
            continue;
        }

        // Build a name for the node
        let name = if let Some(type_name) = header.meta.get_type_name() {
            let mut name = type_name.type_name.clone();
            if opts.no_short_names == false {
                for short in type_name.type_name.split(":") {
                    name = short.to_string();
                }
            }
            if let Some(key) = header.meta.get_data_key() {
                format!("{}(key={})", name.bold(), key)
            } else {
                name.as_str().bold().to_string()
            }
        } else if let Some(tombstone) = header.meta.get_tombstone() {
            if opts.no_compact == false {
                tree_lookup.remove(&tombstone);
                continue;
            }
            format!("{}({})", "tombstone".yellow(), tombstone)
        } else if let Some(pk) = header.meta.get_public_key() {
            format!("public-key({})", pk.hash())
        } else {
            "unknown".bold().to_string()
        };

        // Insert some data into the node
        let key = match header.meta.get_data_key() {
            Some(a) => a,
            None => PrimaryKey::generate(),
        };
        let node = tree_lookup.entry(key.clone()).or_insert_with(|| {
            let mut node = EventNode::default();
            node.name = name.clone();
            node
        });

        // Put the actual data in the node
        if opts.no_compact == false {
            node.versions.clear();
        }
        if header.meta.get_tombstone().is_some() {
            if opts.no_compact == false {
                drop(node);
                tree_lookup.remove(&key);
                continue;
            }
        }

        // Put the actual data in the node
        let d = tree_event.entry(header.raw.event_hash.clone()).or_default();
        d.data = if header.raw.data_size > 0 {
            Some(format!("{}({} bytes)", "data", header.raw.data_size))
        } else {
            Some(format!("{}", name))
        };
        d.data_hash = header.raw.data_hash;
        d.event_hash = Some(header.raw.event_hash);
        if let Some(sig) = &d.sig {
            if hash_pk.contains(sig) == false {
                d.bad_pk = true;
            }
        } else {
            d.bad_order = true;
        }
        node.versions.push(header.raw.event_hash.clone());

        // Put the node in the right place
        match header.meta.get_parent() {
            Some(a) => {
                if let Some(a) = tree_lookup.get_mut(&a.vec.parent_id) {
                    if a.children.iter().any(|a| a.eq(&key)) == false {
                        a.children.push(key);
                    }
                }
            }
            None => {
                if tree_roots.iter().any(|a| a.eq(&key)) == false {
                    tree_roots.push(key);
                }
            }
        }
    }

    // Turn it into ascii-tree
    let mut output = String::new();
    for tree_pk in tree_pks {
        let tree = Tree::Node(format!("public-key({})", tree_pk.to_8hex()), Vec::default());
        ascii_tree::write_tree(&mut output, &tree).unwrap();
    }
    for tree_sig in tree_sigs {
        let no_compact = opts.no_compact;
        let mut data = tree_sig
            .hashes
            .into_iter()
            .filter_map(|d| {
                if tree_event.contains_key(&d) {
                    match no_compact {
                        true => Some(format!("{}({})", "event", d.to_8hex())),
                        false => None,
                    }
                } else {
                    Some(format!("{}({}) {}", "event", d.to_8hex(), "missing".red()))
                }
            })
            .collect::<Vec<_>>();
        if data.len() <= 0 {
            continue;
        }
        data.insert(0, format!("sig-data({} bytes)", tree_sig.signature.len()));
        let name = match hash_pk.contains(&tree_sig.public_key_hash) {
            true => format!(
                "signature({}) {}",
                "pk-ref".green(),
                tree_sig.public_key_hash.to_8hex()
            ),
            false => format!(
                "signature({}) {}",
                "pk-missing".red(),
                tree_sig.public_key_hash.to_8hex()
            ),
        };
        let tree = Tree::Node(name, vec![Tree::Leaf(data)]);
        ascii_tree::write_tree(&mut output, &tree).unwrap();
    }
    for root in tree_roots.iter() {
        let tree = build_tree(root, &tree_lookup, &tree_event);
        if let Some(tree) = tree {
            ascii_tree::write_tree(&mut output, &tree).unwrap();
        }
    }
    print!("{}", output);

    Ok(())
}

fn build_tree(
    key: &PrimaryKey,
    tree_lookup: &FxHashMap<PrimaryKey, EventNode>,
    tree_data: &FxHashMap<AteHash, EventData>,
) -> Option<Tree> {
    if let Some(node) = tree_lookup.get(&key) {
        let mut children = Vec::new();
        if node.versions.len() > 0 {
            let versions = node
                .versions
                .iter()
                .filter_map(|a| tree_data.get(a))
                .map(|a| {
                    let e = a
                        .event_hash
                        .map_or_else(|| "none".to_string(), |f| f.to_8hex());
                    match &a.data {
                        Some(b) => (b.clone(), a, a.data_hash.clone(), e),
                        None => ("missing".to_string(), a, a.data_hash.clone(), e),
                    }
                })
                .map(|(d, a, h, e)| {
                    if let Some(s) = a.sig.clone() {
                        if a.bad_order {
                            format!(
                                "{} {}({}) evt={}",
                                d.yellow(),
                                "sig-bad-order".red(),
                                s.to_8hex(),
                                e
                            )
                        } else if a.bad_pk {
                            format!(
                                "{} {}({}) evt={}",
                                d.yellow(),
                                "sig-bad-pk".red(),
                                s.to_8hex(),
                                e
                            )
                        } else {
                            format!("{} {}({}) evt={}", d, "sig".green(), s.to_8hex(), e)
                        }
                    } else if let Some(h) = h {
                        format!(
                            "{} {}({}) evt={}",
                            d.yellow(),
                            "no-sig".red(),
                            h.to_8hex(),
                            e
                        )
                    } else {
                        format!("{} {}, evt={}", d.yellow(), "no-sig".red(), e)
                    }
                })
                .collect::<Vec<_>>();
            let leaf = Tree::Leaf(versions);
            children.push(leaf);
        }
        for c in &node.children {
            if let Some(c) = build_tree(c, tree_lookup, tree_data) {
                children.push(c);
            }
        }
        Some(Tree::Node(node.name.clone(), children))
    } else {
        None
    }
}
