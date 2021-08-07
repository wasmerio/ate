#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tracing_futures::{Instrument};
use error_chain::bail;

use multimap::MultiMap;
use btreemultimap::BTreeMultiMap;
use tokio::sync::broadcast;

use crate::error::*;
use crate::conf::*;
use crate::index::*;
use crate::transaction::*;
use crate::compact::*;

use std::sync::{Arc};
use parking_lot::Mutex as StdMutex;
use fxhash::{FxHashSet};
use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;

use crate::redo::*;
use crate::spec::SerializationFormat;
use crate::time::TimeKeeper;
use crate::trust::*;
use crate::pipe::*;
use crate::loader::*;
use crate::event::EventHeader;
use crate::engine::TaskEngine;

use crate::trust::ChainKey;

use super::*;
use super::inbox_pipe::*;
use super::workers::ChainWorkProcessor;

impl<'a> Chain
{
    #[allow(dead_code)]
    pub(crate) async fn new(
        builder: ChainBuilder,
        key: &ChainKey,
    ) -> Result<Chain, ChainCreationError>
    {
        Chain::new_ext(builder, key.clone(), None, true)
            .await
    }

    #[allow(dead_code)]
    pub async fn new_ext(
        builder: ChainBuilder,
        key: ChainKey,
        extra_loader: Option<Box<dyn Loader>>,
        allow_process_errors: bool,
    ) -> Result<Chain, ChainCreationError>
    {
        debug!("open: {}", key);

        // Compute the open flags
        #[cfg(feature = "enable_local_fs")]
        let flags = OpenFlags {
            truncate: builder.truncate,
            temporal: builder.temporal,
            integrity: builder.integrity,
        };
        let compact_mode = builder.cfg_ate.compact_mode;
        let compact_bootstrap = builder.cfg_ate.compact_bootstrap;
        
        // Create a redo log loader which will listen to all the events as they are
        // streamed in and extract the event headers
        #[cfg(feature = "enable_local_fs")]
        let (loader, mut rx) = RedoLogLoader::new();

        // We create a composite loader that includes any user defined loader
        let mut composite_loader = Box::new(crate::loader::CompositionLoader::default());
        #[cfg(feature = "enable_local_fs")]
        composite_loader.loaders.push(loader);
        if let Some(a) = extra_loader {
            composite_loader.loaders.push(a);
        }

        // Build the header
        let header = ChainHeader::default();
        let header_bytes = SerializationFormat::Json.serialize(&header)?;
        
        // Create the redo log itself which will open the files and stream in the events
        // in a background thread
        #[cfg(feature = "enable_local_fs")]
        let redo_log = {
            let key = key.clone();
            let builder = builder.clone();
            async move {
                RedoLog::open_ext(&builder.cfg_ate, &key, flags, composite_loader, header_bytes).await
            }
        };
        #[cfg(not(feature = "enable_local_fs"))]
        let redo_log = {
            async move {
                RedoLog::open(header_bytes).await
            }
        };
        
        // While the events are streamed in we build a list of all the event headers
        // but we strip off the data itself
        let process_local = async move {
            #[allow(unused_mut)]
            let mut headers = Vec::new();
            #[cfg(feature = "enable_local_fs")]
            while let Some(result) = rx.recv().await {
                headers.push(result.header.as_header()?);
            }
            Result::<Vec<EventHeader>, SerializationError>::Ok(headers)
        };
        
        // Join the redo log thread earlier after the events were successfully streamed in
        let (redo_log, process_local) = futures::join!(redo_log, process_local);
        let headers = process_local?;
        let redo_log = redo_log?;

        // Construnct the chain-of-trust on top of the redo-log
        let chain = ChainOfTrust {
            debug_id: fastrand::u64(..),
            key: key.clone(),
            redo: redo_log,
            timeline: ChainTimeline {
                history: BTreeMultiMap::new(),
                pointers: BinaryTreeIndexer::default(),
                compactors: builder.compactors,
            },
        };

        // Construct all the protected fields that are behind a synchronous critical section
        // that does not wait
        let mut inside_sync = ChainProtectedSync {
            sniffers: Vec::new(),
            services: Vec::new(),
            indexers: builder.indexers,
            plugins: builder.plugins,
            linters: builder.linters,
            validators: builder.validators,
            transformers: builder.transformers,
            default_session: builder.session,
            integrity: builder.integrity,
        };

        // Add a tree authority plug if one is in the builder
        if let Some(tree) = builder.tree {
            inside_sync.plugins.push(Box::new(tree));
        }

        // Set the integrity mode on all the validators
        inside_sync.set_integrity_mode(builder.integrity);

        // Wrap the sync object
        let inside_sync
            = Arc::new(StdRwLock::new(inside_sync));

        // Create an exit watcher
        let (exit_tx, _) = broadcast::channel(1);

        // The asynchronous critical section protects the chain-of-trust itself and
        // will have longer waits on it when there are writes occuring
        let mut inside_async = ChainProtectedAsync {
            chain,
            default_format: builder.cfg_ate.log_format,
            disable_new_roots: false,
            sync_tolerance: builder.cfg_ate.sync_tolerance,
            listeners: MultiMap::new(),
        };

        // Check all the process events
        #[cfg(feature = "enable_verbose")]
        for a in headers.iter() {
            match a.meta.get_data_key() {
                Some(key) => debug!("loaded: {} data {}", a.raw.event_hash, key),
                None => debug!("loaded: {}", a.raw.event_hash)
            }
        }
        
        // Process all the events in the chain-of-trust
        let conversation = Arc::new(ConversationSession::new(true));
        if let Err(err) = inside_async.process(inside_sync.write(), headers, Some(&conversation)) {
            if allow_process_errors == false {
                return Err(err);
            }
        }
        
        // Create the compaction state (which later we will pass to the compaction thread)
        let (compact_tx, compact_rx) = CompactState::new(compact_mode, inside_async.chain.redo.size() as u64);

        // Make the inside async immutable
        let inside_async = Arc::new(RwLock::new(inside_async));

        // The worker thread processes events that come in
        let worker_inside_async = Arc::clone(&inside_async);
        let worker_inside_sync = Arc::clone(&inside_sync);

        // background thread - receives events and processes them
        let sender = ChainWorkProcessor::new(worker_inside_async, worker_inside_sync, compact_tx);

        // decache subscription
        let (decache_tx, _) = broadcast::channel(1000);

        // The inbox pipe intercepts requests to and processes them
        let mut pipe: Arc<Box<dyn EventPipe>> = Arc::new(Box::new(InboxPipe {
            inbox: sender,
            decache: decache_tx.clone(),
            locks: StdMutex::new(FxHashSet::default()),
        }));
        if let Some(second) = builder.pipes {
            pipe = Arc::new(Box::new(DuelPipe::new(second, pipe)));
        };

        // Create the NTP worker thats needed to build the timeline
        let tolerance = builder.configured_for.ntp_tolerance();
        let time = Arc::new(TimeKeeper::new(&builder.cfg_ate, tolerance).await?);

        // Create the chain that will be returned to the caller
        let chain = Chain {
            key: key.clone(),
            node_id: builder.node_id.clone(),
            cfg_ate: builder.cfg_ate.clone(),
            remote_addr: None,
            default_format: builder.cfg_ate.log_format,
            inside_sync,
            inside_async,
            pipe,
            time,
            exit: exit_tx.clone(),
            decache: decache_tx,
        };

        // If we are to compact the log on bootstrap then do so
        debug!("compact-now: {}", compact_bootstrap);
        if compact_bootstrap {
            chain.compact().await?;
        }

        // Start the compactor worker thread on the chain
        if builder.cfg_ate.compact_mode != CompactMode::Never {
            debug!("compact-mode-on: {}", builder.cfg_ate.compact_mode);

            let worker_exit = exit_tx.subscribe();
            let worker_inside_async = Arc::clone(&chain.inside_async);
            let worker_inside_sync = Arc::clone(&chain.inside_sync);
            let worker_pipe = Arc::clone(&chain.pipe);
            let time = Arc::clone(&chain.time);

            // background thread - periodically compacts the chain into a smaller memory footprint
            TaskEngine::spawn(Chain::worker_compactor(worker_inside_async, worker_inside_sync, worker_pipe, time, compact_rx, worker_exit));
        } else {
            debug!("compact-mode-off: {}", builder.cfg_ate.compact_mode);
        }

        // Create the chain
        Ok(
            chain
        )
    }
}