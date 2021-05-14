#[allow(unused_imports)]
use log::{info, error, debug};

use multimap::MultiMap;
use btreemultimap::BTreeMultiMap;

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
use tokio::sync::mpsc;

use crate::redo::*;
use crate::spec::SerializationFormat;
use crate::time::TimeKeeper;
use crate::trust::*;
use crate::pipe::*;
use crate::loader::*;

use crate::trust::ChainKey;

use super::*;
use super::inbox_pipe::*;

impl<'a> Chain
{
    #[allow(dead_code)]
    pub(crate) async fn new(
        builder: ChainBuilder,
        key: &ChainKey,
    ) -> Result<Chain, ChainCreationError>
    {
        Chain::new_ext(builder, key.clone(), None, true).await
    }

    #[allow(dead_code)]
    pub async fn new_ext(
        builder: ChainBuilder,
        key: ChainKey,
        extra_loader: Option<Box<dyn Loader>>,
        allow_process_errors: bool,
    ) -> Result<Chain, ChainCreationError>
    {
        // Compute the open flags
        let flags = OpenFlags {
            truncate: builder.truncate,
            temporal: builder.temporal,
            integrity: builder.integrity,
        };
        let compact_mode = builder.cfg.compact_mode;
        let compact_bootstrap = builder.cfg.compact_bootstrap;
        
        // Create a redo log loader which will listen to all the events as they are
        // streamed in and extract the event headers
        let (loader, mut rx) = RedoLogLoader::new();

        // We create a composite loader that includes any user defined loader
        let mut composite_loader = Box::new(crate::loader::CompositionLoader::default());
        composite_loader.loaders.push(loader);
        if let Some(a) = extra_loader {
            composite_loader.loaders.push(a);
        }

        // Build the header
        let header = ChainHeader::default();
        let header_bytes = SerializationFormat::Json.serialize(&header)?;
        
        // Create the redo log itself which will open the files and stream in the events
        // in a background thread
        let redo_log = {
            let key = key.clone();
            let builder = builder.clone();
            tokio::spawn(async move {
                RedoLog::open_ext(&builder.cfg, &key, flags, composite_loader, header_bytes).await
            })
        };
        
        // While the events are streamed in we build a list of all the event headers
        // but we strip off the data itself
        let mut headers = Vec::new();
        while let Some(result) = rx.recv().await {
            headers.push(result.header.as_header()?);
        }

        // Join the redo log thread earlier after the events were successfully streamed in
        let redo_log = redo_log.await.unwrap()?;

        // Construnct the chain-of-trust on top of the redo-log
        let chain = ChainOfTrust {
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
            indexers: builder.indexers,
            plugins: builder.plugins,
            linters: builder.linters,
            validators: builder.validators,
            transformers: builder.transformers,
            listeners: MultiMap::new(),
            services: Vec::new(),
            repository: None,
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

        // The asynchronous critical section protects the chain-of-trust itself and
        // will have longer waits on it when there are writes occuring
        let mut inside_async = ChainProtectedAsync {
            chain,
            default_format: builder.cfg.log_format,
            disable_new_roots: false,
            run: true,
        };
        
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

        // We create a channel that will be used to feed events from the inbox pipe into
        // the chain of trust itself when writes occur locally or are received on the network
        let (sender,
             receiver)
             = mpsc::channel(builder.cfg.buffer_size_client);

        // The worker thread processes events that come in
        let worker_inside_async = Arc::clone(&inside_async);
        let worker_inside_sync = Arc::clone(&inside_sync);
        tokio::task::spawn(Chain::worker_receiver(worker_inside_async, worker_inside_sync, receiver, compact_tx));

        // The inbox pipe intercepts requests to and processes them
        let mut pipe: Arc<Box<dyn EventPipe>> = Arc::new(Box::new(InboxPipe {
            inbox: sender,
            locks: StdMutex::new(FxHashSet::default()),
        }));
        if let Some(second) = builder.pipes {
            pipe = Arc::new(Box::new(DuelPipe::new(second, pipe)));
        };

        // Create the NTP worker thats needed to build the timeline
        let tolerance = builder.configured_for.ntp_tolerance();
        let time = Arc::new(TimeKeeper::new(&builder.cfg, tolerance).await?);

        // Create the chain that will be returned to thecaller
        let chain = Chain {
            key: key.clone(),
            default_format: builder.cfg.log_format,
            inside_sync,
            inside_async,
            pipe,
            time,
        };

        // If we are to compact the log on bootstrap then do so
        if compact_bootstrap {
            chain.compact().await?;
        }

        // Start the compactor worker thread on the chain
        let worker_inside_async = Arc::clone(&chain.inside_async);
        let worker_inside_sync = Arc::clone(&chain.inside_sync);
        let worker_pipe = Arc::clone(&chain.pipe);
        tokio::task::spawn(Chain::worker_compactor(worker_inside_async, worker_inside_sync, worker_pipe, compact_rx));

        // Create the chain
        Ok(
            chain
        )
    }
}