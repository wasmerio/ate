#![allow(unused_imports)]
#![allow(dead_code)]
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use std::borrow::Cow;
use derivative::*;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::cconst::*;
use super::common::*;
use super::err;
use super::fd::*;
use super::job::*;
use super::reactor::*;
use super::stdout::*;
use super::pipe::*;
use super::api::*;

#[derive(Debug, Clone)]
pub enum TtyMode {
    Null,
    Console,
    StdIn(Job),
}

struct TtyInnerAsync {
    pub line: String,
    pub paragraph: String,
    pub cursor_pos: usize,
    pub cursor_history: usize,
    pub history: Vec<String>,
    pub mode: TtyMode,
    pub echo: bool,
    pub prompt: String,
    pub prompt_color: String,
    pub cols: u32,
    pub rows: u32,
}

#[derive(Debug)]
struct TtyInnerSync {
    pub buffering: AtomicBool,
    pub raw: AtomicBool,
}

impl TtyInnerAsync {
    pub fn reset_line(&mut self) {
        self.line.clear();
        self.cursor_pos = 0;
    }

    pub fn reset_history_cursor(&mut self) {
        self.cursor_history = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TtyOuter {
    Normal,
    Mobile,
    SSH,
}

impl TtyOuter {
    pub fn is_mobile(&self) -> bool {
        use TtyOuter::*;
        match self {
            Normal => false,
            Mobile => true,
            SSH => false    
        }
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Tty {
    #[derivative(Debug = "ignore")]
    inner_async: Arc<AsyncMutex<TtyInnerAsync>>,
    inner_sync: Arc<TtyInnerSync>,
    stdout: Stdout,
    stderr: Fd,
    log: Fd,
    outer: TtyOuter,
}

impl Tty {
    pub fn new(stdout: Stdout, stderr: Fd, log: Fd, outer: TtyOuter) -> Tty {
        let mut stdout = stdout.clone();
        stdout.set_flag(FdFlag::Stdout(true));
        Tty {
            inner_async: Arc::new(AsyncMutex::new(TtyInnerAsync {
                line: String::new(),
                paragraph: String::new(),
                cursor_pos: 0,
                cursor_history: 0,
                history: Vec::new(),
                mode: TtyMode::Console,
                echo: true,
                prompt: "$".to_string(),
                prompt_color: "$".to_string(),
                cols: 1,
                rows: 1,
            })),
            inner_sync: Arc::new(TtyInnerSync {
                buffering: AtomicBool::new(true),
                raw: AtomicBool::new(false)
            }),
            stdout,
            stderr,
            log,
            outer,
        }
    }

    pub fn channel(abi: &Arc<dyn ConsoleAbi>, unfinished_line: &Arc<AtomicBool>, outer: TtyOuter) -> Tty {
        let (stdio, mut stdio_rx) = pipe_out(FdFlag::None);
        let mut stdout = stdio.clone();
        let mut stderr = stdio.clone();
        let mut log = stdio.clone();
        stdout.set_flag(FdFlag::Stdout(true));
        stderr.set_flag(FdFlag::Stderr(true));
        log.set_flag(FdFlag::Log);
        let stdout = Stdout::new(stdout);
        let tty = Tty::new(stdout, stderr, log, outer);

        // Stdout, Stderr and Logging (this is serialized in order for a good reason!)
        let unfinished_line = unfinished_line.clone();
        let system = System::default();
        {
            let abi = abi.clone();
            let work = async move {
                while let Some(msg) = stdio_rx.recv().await {
                    match msg {
                        FdMsg::Data { data, flag } => match flag {
                            FdFlag::Log => {
                                let txt = String::from_utf8_lossy(&data[..]);
                                let mut txt = txt.as_ref();
                                while txt.ends_with("\n") || txt.ends_with("\r") {
                                    txt = &txt[..(txt.len() - 1)];
                                }
                                abi.log(txt.to_string()).await;
                            }
                            _ => {
                                let text =
                                    String::from_utf8_lossy(&data[..])[..].replace("\n", "\r\n");
                                match flag {
                                    FdFlag::Stderr(_) => abi.stderr(text.as_bytes().to_vec()).await,
                                    _ => abi.stdout(text.as_bytes().to_vec()).await,
                                };

                                let is_unfinished = is_cleared_line(&text) == false;
                                unfinished_line.store(is_unfinished, Ordering::Release);
                            }
                        },
                        FdMsg::Flush { tx } => {
                            let _ = tx.send(()).await;
                        }
                    }
                }
                info!("main IO loop exited");
            };
            #[cfg(target_family = "wasm")]
            system.fork_local(work);
            #[cfg(not(target_family = "wasm"))]
            system.fork_shared(move || work);
        }

        tty
    }

    pub fn stdout(&self) -> Stdout {
        self.stdout.clone()
    }

    pub fn fd_stdout(&self) -> Fd {
        self.stdout.fd.clone()
    }

    pub fn stderr(&self) -> Fd {
        self.stderr.clone()
    }

    pub fn log(&self) -> Fd {
        self.log.clone()
    }

    pub async fn reset_line(&self) {
        self.inner_async.lock().await.reset_line();
    }

    pub async fn get_selected_history(&self) -> Option<String> {
        let inner = self.inner_async.lock().await;
        if inner.cursor_history > inner.history.len() {
            return None;
        }
        let cursor_history = inner.history.len() - inner.cursor_history;
        inner.history.get(cursor_history).map(|a| a.clone())
    }

    pub async fn restore_selected_history(&mut self) {
        let cursor_history = {
            let mut inner = self.inner_async.lock().await;
            if inner.cursor_history > inner.history.len() {
                inner.reset_line();
                debug!("restore-history-over");
                return;
            }
            inner.history.len() - inner.cursor_history
        };

        self.set_cursor_to_start().await;
        self.draw_undo().await;

        let right = {
            let mut inner = self.inner_async.lock().await;
            let last = inner.history.get(cursor_history).map(|a| a.clone());
            if let Some(last) = last {
                debug!("restore-history: pos={} val={}", cursor_history, last);
                inner.cursor_pos = last.len();
                inner.line = last.clone();
                last as String
            } else {
                inner.reset_line();
                debug!("restore-history: pos={} miss", cursor_history);
                String::new()
            }
        };

        self.draw(right.as_str()).await
    }

    pub async fn record_history(&self, cmd: String) {
        if cmd.len() <= 0 {
            return;
        }

        let mut inner = self.inner_async.lock().await;
        debug!("add-history: {}", cmd);
        inner.history.retain(|c| c.ne(&cmd));
        inner.history.push(cmd);
    }

    pub async fn get_paragraph(&self) -> String {
        let mut inner = self.inner_async.lock().await;
        if inner.line.len() <= 0 {
            return String::new();
        }

        if inner.paragraph.len() > 0 {
            inner.paragraph += " ";
        }
        let line = inner.line.clone();
        inner.paragraph += line.as_str();
        inner.paragraph.clone()
    }

    pub async fn reset_history_cursor(&self) {
        let mut inner = self.inner_async.lock().await;
        inner.reset_history_cursor();
    }

    pub async fn reset_paragraph(&self) {
        let mut inner = self.inner_async.lock().await;
        inner.paragraph.clear();
    }

    pub async fn set_bounds(&self, cols: u32, rows: u32) {
        let mut inner = self.inner_async.lock().await;
        inner.cols = cols;
        inner.rows = rows;
    }

    pub async fn backspace(&mut self) {
        let echo = {
            let inner = self.inner_async.lock().await;
            if inner.cursor_pos <= 0 {
                return;
            }
            inner.echo
        };

        if echo {
            self.draw(Tty::TERM_CURSOR_LEFT).await;
            self.draw_undo().await;
        }
        let right = {
            let mut inner = self.inner_async.lock().await;
            let left = inner.line[..inner.cursor_pos - 1].to_string();
            let right = inner.line[inner.cursor_pos..].to_string();
            inner.line = format!("{}{}", left, right);
            inner.cursor_pos -= 1;
            right
        };
        if echo {
            self.draw_fixed(right.as_str()).await
        }
    }

    pub async fn delete(&mut self) {
        let echo = {
            let inner = self.inner_async.lock().await;
            if inner.cursor_pos >= inner.line.len() {
                return;
            }
            inner.echo
        };

        if echo {
            self.draw_undo().await;
        }
        let right = {
            let mut inner = self.inner_async.lock().await;
            let left = inner.line[..inner.cursor_pos].to_string();
            let right = inner.line[inner.cursor_pos + 1..].to_string();
            inner.line = format!("{}{}", left, right);
            right
        };
        if echo {
            self.draw_fixed(right.as_str()).await
        }
    }

    pub async fn cursor_left(&mut self) {
        let echo = {
            let mut inner = self.inner_async.lock().await;
            if inner.cursor_pos <= 0 {
                return;
            }
            inner.cursor_pos -= 1;
            inner.echo
        };
        if echo {
            self.draw(Tty::TERM_CURSOR_LEFT).await;
        }
    }

    pub async fn cursor_right(&mut self) {
        let echo = {
            let mut inner = self.inner_async.lock().await;
            if inner.cursor_pos >= inner.line.len() {
                return;
            }
            inner.cursor_pos += 1;
            inner.echo
        };
        if echo {
            self.draw(Tty::TERM_CURSOR_RIGHT).await;
        }
    }

    pub async fn cursor_up(&mut self) {
        let _echo = {
            let mut inner = self.inner_async.lock().await;
            if inner.cursor_history < inner.history.len() {
                inner.cursor_history += 1;
            }
            inner.echo
        };
        self.restore_selected_history().await;
    }

    pub async fn cursor_down(&mut self) {
        let _echo = {
            let mut inner = self.inner_async.lock().await;
            if inner.cursor_history > 0 {
                inner.cursor_history -= 1;
            }
            inner.echo
        };
        self.restore_selected_history().await;
    }

    pub async fn enter_mode(&self, mut mode: TtyMode, reactor: &Arc<RwLock<Reactor>>) {
        self.set_buffering(true);

        let last_mode = {
            let mut inner = self.inner_async.lock().await;
            std::mem::swap(&mut inner.mode, &mut mode);
            mode
        };

        let mut reactor = reactor.write().await;
        match last_mode {
            TtyMode::StdIn(job) => {
                reactor.close_job(job, std::num::NonZeroU32::new(err::ERR_TERMINATED).unwrap());
            }
            _ => {}
        }
    }

    pub fn set_buffering(&self, on: bool) {
        debug!("set_buffering on={}", on);
        self.inner_sync.buffering.store(on, Ordering::Relaxed);
    }

    pub fn is_buffering(&self) -> bool {
        self.inner_sync.buffering.load(Ordering::Relaxed)
    }

    /// Raw mode will send the bytes to the STDIN without doing
    /// anything to them
    pub fn set_raw_mode(&self, on: bool) {
        debug!("set_raw on={}", on);
        self.inner_sync.raw.store(on, Ordering::Relaxed);
    }

    pub fn is_raw_mode(&self) -> bool {
        self.inner_sync.raw.load(Ordering::Relaxed)
    }

    pub async fn set_prompt(&self, prompt: String, prompt_color: String) {
        let mut inner = self.inner_async.lock().await;
        inner.prompt = prompt;
        inner.prompt_color = prompt_color;
    }

    pub async fn mode(&self) -> TtyMode {
        let inner = self.inner_async.lock().await;
        inner.mode.clone()
    }

    pub async fn mode_mut<F>(&self, mut func: F)
    where F: FnMut(&mut TtyMode) {
        let mut inner = self.inner_async.lock().await;
        func(&mut inner.mode);
    }

    pub async fn echo(&mut self, data: &str) {
        let echo = self.inner_async.lock().await.echo;
        if echo {
            self.draw(data).await;
        }
    }

    pub async fn set_echo(&self, echo: bool) {
        self.inner_async.lock().await.echo = echo;
    }

    pub async fn add(&mut self, data: &str) {
        let echo = self.inner_async.lock().await.echo;
        if echo {
            self.draw_undo().await;
        }
        let right = {
            let mut inner = self.inner_async.lock().await;
            let cursor_pos = inner.cursor_pos;
            inner.line.insert_str(cursor_pos, data);
            inner.cursor_pos += data.len();

            let right = if inner.cursor_pos < inner.line.len() {
                Some(inner.line[inner.cursor_pos..].to_string())
            } else {
                None
            };
            right
        };
        if echo {
            let mut chars = String::new();
            chars += Tty::TERM_WRAPAROUND;
            chars += data;
            if let Some(right) = right {
                chars += Tty::TERM_CURSOR_SAVE;
                chars += right.as_str();
                chars += Tty::TERM_CURSOR_RESTORE;
            }
            self.draw(chars.as_str()).await;
        }
    }

    pub async fn draw_prompt(&mut self) {
        let prompt_color = self.inner_async.lock().await.prompt_color.clone();
        let mut chars = String::new();
        chars += Tty::TERM_DELETE_BELOW;
        chars += Tty::TERM_DELETE_LINE;
        chars += Tty::TERM_WRAPAROUND;
        chars += prompt_color.as_str();
        self.draw(chars.as_str()).await;
    }

    pub async fn draw_welcome(&mut self) {
        let welcome = match self.outer {
            TtyOuter::Normal => Tty::WELCOME,
            TtyOuter::SSH => Tty::WELCOME_MEDIUM,
            TtyOuter::Mobile => Tty::WELCOME_SMALL
        };
        let mut data = welcome
            .replace("\\x1B", "\x1B")
            .replace("\\r", "\r")
            .replace("\\n", "\n");
        data.insert_str(0, Tty::TERM_NO_WRAPAROUND);
        self.draw(data.as_str()).await;
    }

    pub async fn set_cursor_to_start(&mut self) {
        let shift_left = {
            let mut inner = self.inner_async.lock().await;
            let pos = inner.cursor_pos;
            inner.cursor_pos = 0;
            pos
        };

        let chars = std::iter::repeat(Tty::TERM_CURSOR_LEFT)
            .take(shift_left)
            .collect::<String>();
        if chars.len() > 0 {
            self.draw(chars.as_str()).await
        }
    }

    pub async fn set_cursor_to_end(&mut self) {
        let shift_right = {
            let mut inner = self.inner_async.lock().await;
            let pos = inner.cursor_pos;
            if inner.line.len() > 0 {
                inner.cursor_pos = inner.line.len();
                inner.line.len() - pos
            } else {
                inner.cursor_pos = 0;
                0
            }
        };

        let chars = std::iter::repeat(Tty::TERM_CURSOR_RIGHT)
            .take(shift_right)
            .collect::<String>();
        if chars.len() > 0 {
            self.draw(chars.as_str()).await
        }
    }

    pub async fn draw_undo(&mut self) -> String {
        let mut chars = String::new();
        chars += Tty::TERM_CURSOR_SAVE;
        chars += Tty::TERM_DELETE_RIGHT;
        chars += Tty::TERM_CURSOR_RESTORE;
        chars += Tty::TERM_DELETE_BELOW;
        chars += Tty::TERM_CURSOR_RESTORE;
        self.draw(chars.as_str()).await;
        let inner = self.inner_async.lock().await;
        inner.line[inner.cursor_pos..].to_string()
    }

    pub async fn draw_fixed(&mut self, data: &str) {
        let mut chars = String::new();
        chars += Tty::TERM_CURSOR_SAVE;
        chars += Tty::TERM_WRAPAROUND;
        chars += data;
        chars += Tty::TERM_CURSOR_RESTORE;
        self.stdout.draw(chars.as_str()).await;
    }

    pub async fn draw(&mut self, data: &str) {
        self.stdout.draw(data).await;
    }

    pub async fn flush_async(&mut self) -> std::io::Result<()> {
        self.stdout.flush_async().await
    }
}
