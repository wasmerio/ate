#[allow(unused_imports)]
use tracing::{info, error, warn, debug};
use std::{sync::Weak};
use tokio::sync::mpsc;
use std::sync::Arc;
use parking_lot::RwLockReadGuard as StdRwLockReadGuard;

use crate::{error::*, event::*};
use crate::chain::*;
use crate::header::*;

use super::*;

pub(crate) fn callback_events_prepare(guard: &StdRwLockReadGuard<ChainProtectedSync>, events: &Vec<EventData>) -> Vec<Notify>
{
    let mut ret = Vec::new();

    for sniffer in guard.sniffers.iter() {
        if let Some(key) = events.iter().filter_map(|e| match (*sniffer.filter)(e) {
            true => e.meta.get_data_key(),
            false => None,
        }).next() {
            ret.push(sniffer.convert(key));
        }
    }

    for service in guard.services.iter() {
        for key in events.iter().filter(|e| service.filter(&e)).filter_map(|e| e.meta.get_data_key()) {
            ret.push(Notify {
                key,
                who: NotifyWho::Service(Arc::clone(service))
            });
        }
    }

    ret
}

pub(crate) async fn callback_events_notify(mut notifies: Vec<Notify>) -> Result<(), InvokeError>
{
    let mut joins = Vec::new();
    for notify in notifies.drain(..) {
        joins.push(notify.notify());
    }
    for notify in futures::future::join_all(joins).await {
        if let Err(err) = notify {
            #[cfg(debug_assertions)]
            warn!("notify-err - {}", err);
            #[cfg(not(debug_assertions))]
            debug!("notify-err - {}", err);
        }
    }
    Ok(())
}

pub(super) struct SniffCommandHandle
{
    id: u64,
    rx: mpsc::Receiver<PrimaryKey>,
    chain: Weak<Chain>,
}

pub(super) fn sniff_for_command_begin(chain: Weak<Chain>, what: Box<dyn Fn(&EventData) -> bool + Send + Sync>) -> SniffCommandHandle
{
    // Create a sniffer
    let id = fastrand::u64(..);
    let (tx, rx) = mpsc::channel(1);
    let sniffer = ChainSniffer {
        id,
        filter: what,
        notify: tx,
    };

    // Insert a sniffer under a lock
    if let Some(chain) = chain.upgrade() {
        let mut guard = chain.inside_sync.write();
        guard.sniffers.push(sniffer);
    }

    SniffCommandHandle {
        id,
        rx,
        chain: Weak::clone(&chain),
    }
}

pub(super) async fn sniff_for_command_finish(mut handle: SniffCommandHandle) -> Option<PrimaryKey>
{
    // Now wait for the response
    let ret = handle.rx.recv().await;

    // Remove the sniffer
    if let Some(chain) = handle.chain.upgrade() {
        let mut guard = chain.inside_sync.write();
        guard.sniffers.retain(|s| s.id != handle.id);
    }

    // Return the result
    ret
}