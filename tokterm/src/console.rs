#![allow(unused_imports)]
#![allow(dead_code)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use xterm_js_rs::{Terminal};
use std::collections::HashMap;
use tokio::sync::mpsc;
use wasm_bindgen::JsCast;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::RwLock;
use std::path::Path;
use wasmer_wasi::vfs::FileSystem;

use crate::tty::TtyMode;

use super::eval::*;
use super::common::*;
use super::fd::*;
use super::state::*;
use super::environment::*;
use super::builtins::*;
use super::pool::ThreadPool as Pool;
use super::err;
use super::stdout::*;
use super::tty::*;
use super::reactor::*;
use super::job::*;
use super::stdio::*;
use super::bin::*;
use super::fs::*;

pub struct Console
{
    terminal: Terminal,
    state: Arc<Mutex<ConsoleState>>,
    bins: BinFactory,
    tok: TokeraSocketFactory,
    tty: Tty,
    pool: Pool,
    reactor: Arc<RwLock<Reactor>>,
    mounts: UnionFileSystem,
    stdout: Stdout,
    stderr: RawFd,
}

impl Console
{
    pub fn new(terminal: Terminal, pool: Pool) -> Console
    {
        let mut reactor = Reactor::new();

        let state = Arc::new(Mutex::new(ConsoleState::new()));
        let (stdout, mut stdout_rx) = reactor.pipe_out().unwrap();
        let (stderr, mut stderr_rx) = reactor.pipe_out().unwrap();

        let stdout = reactor.fd(stdout);
        let stdout = Stdout::new(stdout);

        let reactor = Arc::new(RwLock::new(reactor));

        // Stdout
        {
            let terminal: Terminal = terminal.clone().dyn_into().unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                while let Some(data) = stdout_rx.recv().await {
                    let text = String::from_utf8_lossy(&data[..])[..].replace("\n", "\r\n");
                    terminal.write(text.as_str());
                }
            });
        }

        // Stderr
        {
            let terminal: Terminal = terminal.clone().dyn_into().unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                while let Some(data) = stderr_rx.recv().await {
                    let text = String::from_utf8_lossy(&data[..])[..].replace("\n", "\r\n");
                    terminal.write(text.as_str());
                }
            });
        }

        let tty = Tty::new(stdout.clone());

        let mounts = super::fs::create_root_fs();

        Console {
            terminal,
            bins: BinFactory::new(),
            tok: TokeraSocketFactory::new(&reactor),
            state,
            stdout,
            stderr,
            tty,
            reactor,
            pool,
            mounts,
        }
    }

    pub async fn init(&mut self)
    {
        let cols = self.terminal.get_cols();
        let rows = self.terminal.get_rows();
        self.tty.set_bounds(cols, rows).await;  

        Console::update_prompt(false, &self.state, &self.tty).await;

        self.tty.draw_welcome().await;
        self.tty.draw_prompt().await;        
    }

    pub fn tty(&self) -> &Tty
    {
        &self.tty
    }

    pub async fn on_enter(&mut self)
    {
        let mode = self.tty.mode().await;

        let pushed = self.tty.draw_undo().await;
        self.tty.set_cursor_to_end().await;
        self.tty.draw_fixed(pushed.as_str()).await;
        self.tty.draw("\n\r").await;

        let mut cmd = self.tty.get_paragraph().await;

        if let TtyMode::StdIn(job) = mode {
            cmd += "\n";
            self.tty.reset_line().await;
            self.tty.clear_paragraph().await;
            let _ = job.stdin_tx.send(cmd.into_bytes()).await;
            return;
        }

        if cmd.len() <= 0 {
            self.tty.reset_line().await;
            self.tty.draw_prompt().await;
            return;
        }

        let state = self.state.clone();
        let reactor = self.reactor.clone();
        let pool = self.pool.clone();
        let (env, last_return, path) = {
            let state = self.state.lock().unwrap();
            let env = state.env.clone();
            let last_return = state.last_return;
            let path = state.path.clone();
            (env, last_return, path)
        };

        let (job, stdio) = {
            let mut reactor = reactor.write().await;
            let (stdin, stdin_tx) = match reactor.pipe_in(ReceiverMode::Stream) {
                Ok(a) => a,
                Err(_) => {
                    drop(reactor);
                    self.tty.draw("term: insufficient file handle space\r\n").await;
                    self.tty.reset_line().await;
                    self.tty.draw_prompt().await;
                    return;
                }
            };
            let stdio = Stdio {
                stdin: reactor.fd(stdin),
                stdout: reactor.fd(self.stdout.raw.clone()),
                stderr: reactor.fd(self.stderr.clone()),
                tty: self.tty.clone(),
                tok: self.tok.clone(),
                root: self.mounts.clone(),
            };

            let job = match reactor.generate_job(stdio.clone(), stdin_tx) {
                Ok((_, job)) => job,
                Err(_) => {
                    drop(reactor);
                    self.tty.draw("term: insufficient job space\r\n").await;
                    self.tty.reset_line().await;
                    self.tty.draw_prompt().await;
                    return;
                }
            };
            (job, stdio)
        };

        let ctx = EvalContext {
            env,
            bins: self.bins.clone(),
            job_list: job.job_list_tx.clone(),
            last_return,
            reactor: reactor.clone(),
            pool,
            path,
            input: cmd.clone(),
            stdio
        };
        
        let rx = eval(ctx).await;

        let mut tty = self.tty.clone();
        tty.reset_line().await;
        tty.clear_paragraph().await;
        tty.enter_mode(TtyMode::StdIn(job), &self.reactor).await;

        wasm_bindgen_futures::spawn_local(async move
        {
            let rx = rx.await;
            
            tty.reset_line().await;
            tty.clear_paragraph().await;
            tty.enter_mode(TtyMode::Console, &reactor).await;

            let record_history = if let Some(history) = tty.get_selected_history().await {
                history != cmd
            } else {
                true
            };
            tty.reset_history_cursor().await;
            
            let mut multiline_input = false;
            match rx {
                Ok(EvalPlan::Executed { code, ctx, show_result }) => {
                    debug!("eval executed (code={})", code);
                    if code != 0 && show_result {
                        let mut chars = String::new();
                        chars += err::exit_code_to_message(code);
                        chars += "\r\n";
                        tty.draw(chars.as_str()).await;
                    }
                    {
                        let mut state = state.lock().unwrap();
                        state.last_return = code;
                        state.env = ctx.env;
                        state.path = ctx.path;
                    }
                    if record_history {
                        tty.record_history(cmd).await;
                    }
                },
                Ok(EvalPlan::InternalError) => {
                    debug!("eval internal error");
                    tty.draw("term: internal error\r\n").await;
                }
                Ok(EvalPlan::MoreInput) => {
                    debug!("eval more input");
                    multiline_input = true;
                    tty.add(cmd.as_str()).await;
                }
                Ok(EvalPlan::Invalid) => {
                    debug!("eval invalid");
                    tty.draw("term: invalid command\r\n").await;
                },
                Err(err) => {
                    debug!("eval recv error (err={})", err);
                    tty.draw(format!("term: command failed - {} \r\n", err).as_str()).await;
                }
            };
            tty.reset_line().await;
            Console::update_prompt(multiline_input, &state, &tty).await;
            tty.draw_prompt().await;
        });
    }

    async fn update_prompt(multiline_input: bool, state: &Arc<Mutex<ConsoleState>>, tty: &Tty)
    {
        let (prompt, prompt_color) = {
            let state = state.lock().unwrap();
            let prompt = state.compute_prompt(multiline_input, false);
            let prompt_color = state.compute_prompt(multiline_input, true);
            (prompt, prompt_color)
        };

        tty.set_prompt(prompt, prompt_color).await;
    }

    pub async fn on_ctrl_l(&mut self)
    {
        self.tty.reset_line().await;
        self.tty.draw_prompt().await;
        self.terminal.clear();
    }

    pub async fn on_ctrl_c(&mut self)
    {
        self.tty.draw("^C\r\n").await;
        
        let mode = self.tty.mode().await;
        match mode {
            TtyMode::Null => {
            },
            TtyMode::Console => {
                self.tty.clear_paragraph().await;
                self.tty.reset_line().await;
                Console::update_prompt(false, &self.state, &self.tty).await;
                self.tty.draw_prompt().await;
            },
            TtyMode::StdIn(job) => {
                {
                    let mut reactor = self.reactor.write().await;
                    reactor.close_job(job, err::ERR_TERMINATED);
                }
                self.tty.enter_mode(TtyMode::Null, &self.reactor).await;
            }
        }
    }

    pub async fn on_resize(&mut self)
    {
        let cols = self.terminal.get_cols();
        let rows = self.terminal.get_rows();
        self.tty.set_bounds(cols, rows).await;
    }

    pub async fn on_parse(&mut self, key_code: u32, key: String, alt_key: bool, ctrl_key: bool, meta_key: bool, job: Option<Job>)
    {
        match key_code {
            Self::TERM_KEY_ENTER => {
                self.on_enter().await;
            }
            Self::TERM_KEY_TAB => {
                self.tty.add("    ").await;
            }
            Self::TERM_KEY_BACKSPACE => {
                self.tty.backspace().await;
            }
            Self::TERM_KEY_INSERT => {
            }
            Self::TERM_KEY_DEL => {
                self.tty.delete().await;
            }
            Self::TERM_KEY_LEFT_ARROW => {
                self.tty.cursor_left().await;
            }
            Self::TERM_KEY_RIGHT_ARROW => {
                self.tty.cursor_right().await;
            }
            Self::TERM_KEY_HOME => {
                self.tty.set_cursor_to_start().await;
            }
            Self::TERM_KEY_END => {
                self.tty.set_cursor_to_end().await;
            }
            Self::TERM_KEY_UP_ARROW |
            Self::TERM_KEY_PAGE_UP => {
                if job.is_none() {
                    self.tty.cursor_up().await;
                }
            }
            Self::TERM_KEY_DOWN_ARROW |
            Self::TERM_KEY_PAGE_DOWN => {
                if job.is_none() {
                    self.tty.cursor_down().await;
                }
            }
            Self::TERM_KEY_L if ctrl_key => {
                self.on_ctrl_l().await;
            }
            Self::TERM_KEY_F1 |
            Self::TERM_KEY_F2 |
            Self::TERM_KEY_F3 |
            Self::TERM_KEY_F4 |
            Self::TERM_KEY_F5 |
            Self::TERM_KEY_F6 |
            Self::TERM_KEY_F7 |
            Self::TERM_KEY_F8 |
            Self::TERM_KEY_F9 |
            Self::TERM_KEY_F10 |
            Self::TERM_KEY_F11 |
            Self::TERM_KEY_F12 => {
            }
            _ => {
                if !alt_key && !ctrl_key && !meta_key {
                    if key.len() > 1 {
                        debug!("special key pressed: code={}, str={}", key_code, key);
                    }
                    self.tty.add(key.as_str()).await;
                }
            }
        }
    }

    pub async fn on_key(&mut self, key_code: u32, key: String, alt_key: bool, ctrl_key: bool, meta_key: bool)
    {
        match key_code {
            Self::TERM_KEY_C if ctrl_key => {
                self.on_ctrl_c().await;
            }
            _ => {
                let mode = self.tty.mode().await;
                match mode {
                    TtyMode::StdIn(job) => {
                        if self.tty.is_buffering() {
                            self.on_parse(key_code, key, alt_key, ctrl_key, meta_key, Some(job)).await
                        } else {
                            let key = match key_code {
                                Self::TERM_KEY_ENTER => "\n".to_string(),
                                Self::TERM_KEY_TAB => "\t".to_string(),
                                Self::TERM_KEY_BACKSPACE  |
                                Self::TERM_KEY_INSERT |
                                Self::TERM_KEY_DEL |
                                Self::TERM_KEY_LEFT_ARROW |
                                Self::TERM_KEY_RIGHT_ARROW |
                                Self::TERM_KEY_UP_ARROW |
                                Self::TERM_KEY_DOWN_ARROW |
                                Self::TERM_KEY_HOME |
                                Self::TERM_KEY_END |
                                Self::TERM_KEY_PAGE_UP |
                                Self::TERM_KEY_PAGE_DOWN |
                                Self::TERM_KEY_F1 |
                                Self::TERM_KEY_F2 |
                                Self::TERM_KEY_F3 |
                                Self::TERM_KEY_F4 |
                                Self::TERM_KEY_F5 |
                                Self::TERM_KEY_F6 |
                                Self::TERM_KEY_F7 |
                                Self::TERM_KEY_F8 |
                                Self::TERM_KEY_F9 |
                                Self::TERM_KEY_F10 |
                                Self::TERM_KEY_F11 |
                                Self::TERM_KEY_F12 => {
                                    char::from_u32(key_code)
                                        .map(|a| [a].iter().collect::<String>())
                                        .unwrap_or_else(|| key)
                                },
                                _ => key,
                            };
                            //self.tty.echo(key.as_str()).await;
                            let _ = job.stdin_tx.send(key.into_bytes()).await;
                        }                        
                    }
                    TtyMode::Null => {
                    }
                    TtyMode::Console => {
                        self.on_parse(key_code, key, alt_key, ctrl_key, meta_key, None).await
                    }
                }        
            }
        }
    }
}