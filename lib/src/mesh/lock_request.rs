use async_trait::async_trait;
use log::{warn, debug, info};
use parking_lot::Mutex as StdMutex;
use std::{sync::Arc, sync::Weak};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use std::sync::mpsc as smpsc;
use fxhash::FxHashMap;
use parking_lot::RwLock as StdRwLock;
use std::ops::Rem;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio::time::timeout;

use super::core::*;
use crate::{anti_replay::AntiReplayPlugin, comms::*};
use crate::trust::*;
use crate::chain::*;
use crate::error::*;
use crate::conf::*;
use crate::transaction::*;
use super::msg::*;
use crate::pipe::*;
use crate::header::*;
use crate::spec::*;
use crate::loader::*;
use crate::crypto::*;
use crate::meta::*;
use crate::session::*;
use crate::time::*;

#[derive(Debug, Clone)]
pub(super) struct LockRequest
{
    pub(super) needed: u32,
    pub(super) positive: u32,
    pub(super) negative: u32,
    pub(super) receiver: smpsc::Sender<bool>,
}

impl LockRequest
{
    /// returns true if the vote is finished
    pub(super) fn entropy(&mut self, result: bool) -> bool {
        match result {
            true => self.positive = self.positive + 1,
            false => self.negative = self.negative + 1,
        }

        if self.positive >= self.needed {
            let _ = self.receiver.send(true);
            return true;
        }

        if self.positive + self.negative >= self.needed {
            let _ = self.receiver.send(false);
            return true;
        }

        return false;
    }

    pub(super) fn cancel(&self) {
        let _ = self.receiver.send(false);
    }
}