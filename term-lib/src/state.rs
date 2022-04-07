#![allow(unused_imports)]
#![allow(dead_code)]
use bytes::{Buf, BytesMut};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::sync::Mutex as AsyncMutex;

use crate::common::*;
use crate::fd::*;

use super::environment::Environment;
use super::eval::Process;
use super::eval::*;
use super::fd::*;
use super::fs::*;
use super::poll::*;
use super::reactor::*;
use super::tty::*;

pub struct ConsoleState {
    pub path: String,
    pub user: String,
    pub env: Environment,
    pub last_return: u32,
    pub unfinished_line: Arc<AtomicBool>,
    pub rootfs: UnionFileSystem,
}

impl ConsoleState {
    pub fn new(root: UnionFileSystem, unfinished_line: Arc<AtomicBool>) -> ConsoleState {
        ConsoleState {
            path: "/".to_string(),
            user: "tokera.sh".to_string(),
            env: Environment::default(),
            last_return: 0,
            unfinished_line,
            rootfs: root,
        }
    }

    pub fn clear_mounts(&mut self) {
        self.rootfs.clear();
    }

    pub fn compute_prompt(&self, need_more_text: bool, color: bool) -> String {
        let prompt_symbol = {
            if need_more_text {
                ">".to_string()
            } else {
                "→".to_string()
            }
        };

        let path = Path::new(&self.path);
        let parent_root = if self.path == "/" || path.parent() == Some(Path::new("/")) {
            "/"
        } else {
            ""
        };
        let end_path = path.file_name().map(|s| s.to_str().unwrap()).unwrap_or("");
        if color {
            format!(
                "{}{} {}{}{}{} {}",
                Tty::COL_BLUE,
                prompt_symbol,
                Tty::COL_BOLD,
                parent_root,
                end_path,
                Tty::COL_WHITE,
                Tty::COL_RESET
            )
        } else {
            format!("{} ", prompt_symbol)
        }
    }
}
