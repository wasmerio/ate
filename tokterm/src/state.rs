#![allow(unused_imports)]
#![allow(dead_code)]
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::sync::watch;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use bytes::{Buf, BytesMut};
use tokio::sync::oneshot;

use crate::common::*;
use crate::fd::*;

use super::eval::Process;
use super::environment::Environment;
use super::pool::ThreadPool as Pool;
use super::reactor::*;
use super::poll::*;
use super::fd::*;
use super::eval::*;
use super::tty::*;

pub struct ConsoleState
{
    pub path: String,
    pub user: String,
    pub env: Environment,
    pub last_return: i32,
}

impl ConsoleState
{
    pub fn new() -> ConsoleState {
        ConsoleState {
            path: "/".to_string(),
            user: "wasm.sh".to_string(),
            env: Environment::default(),
            last_return: 0,
        }
    }

    pub fn compute_prompt(&self, need_more_text: bool, color: bool) -> String
    {
        let prompt_symbol = {
            if need_more_text {
                ">".to_string()
            } else {
                "$".to_string()
            }
        };

        if color {
            format!("{}{}{}:{}{}{}{} {}", Tty::COL_GREEN, self.user, Tty::COL_WHITE, Tty::COL_BLUE, self.path, Tty::COL_WHITE, prompt_symbol, Tty::COL_RESET)
        } else {
            format!("{}:{}{} ", self.user, self.path, prompt_symbol)
        }
    }
}