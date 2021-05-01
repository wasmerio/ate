#[allow(unused_imports)]
use log::{info, error, warn, debug};
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

pub(crate) async fn callback_events_notify(mut notifies: Vec<Notify>) -> Result<(), ServiceError<()>>
{
    for notify in notifies.drain(..) {
        tokio::spawn(notify.notify());
    }
    Ok(())
}

pub(super) async fn sniff_for_command(chain: Weak<Chain>, what: Box<dyn Fn(&EventData) -> bool + Send + Sync>) -> Option<PrimaryKey>
{
    // Create a sniffer
    let id = fastrand::u64(..);
    let (tx, mut rx) = mpsc::channel(1);
    let sniffer = ChainSniffer {
        id,
        filter: what,
        notify: tx,
    };

    // Insert a sniffer under a lock
    if let Some(chain) = chain.upgrade() {
        let mut guard = chain.inside_sync.write();
        guard.sniffers.push(sniffer);
    } else {
        return None;
    }

    // Now wait for the response
    let ret = rx.recv().await;

    // Remove the sniffer
    if let Some(chain) = chain.upgrade() {
        let mut guard = chain.inside_sync.write();
        guard.sniffers.retain(|s| s.id != id);
    }

    // Return the result
    ret
}