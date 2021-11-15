use crate::engine::timeout;
use async_trait::async_trait;
use fxhash::FxHashMap;
use std::ops::Rem;
use std::sync::Mutex as StdMutex;
use std::sync::RwLock as StdRwLock;
use std::time::Duration;
use std::time::Instant;
use std::{sync::Arc, sync::Weak};
use tokio::sync::broadcast;
use tokio::sync::watch;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::core::*;
use super::msg::*;
use crate::chain::*;
use crate::conf::*;
use crate::crypto::*;
use crate::error::*;
use crate::header::*;
use crate::loader::*;
use crate::meta::*;
use crate::pipe::*;
use crate::session::*;
use crate::spec::*;
use crate::time::*;
use crate::transaction::*;
use crate::trust::*;
use crate::{anti_replay::AntiReplayPlugin, comms::*};

#[derive(Debug)]
pub(super) struct LockRequest {
    pub(super) needed: u32,
    pub(super) positive: u32,
    pub(super) negative: u32,
    pub(super) tx: watch::Sender<bool>,
}

impl LockRequest {
    /// returns true if the vote is finished
    pub(super) fn entropy(&mut self, result: bool) -> bool {
        match result {
            true => self.positive = self.positive + 1,
            false => self.negative = self.negative + 1,
        }

        if self.positive >= self.needed {
            let _ = self.tx.send(true);
            return true;
        }

        if self.positive + self.negative >= self.needed {
            let _ = self.tx.send(false);
            return true;
        }

        return false;
    }

    pub(super) fn cancel(&self) {
        let _ = self.tx.send(false);
    }
}
