#![allow(unused_imports)]
#![allow(dead_code)]
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::JsCast;
use wasmer_wasi::vfs::FileSystem;
use web_sys::HtmlCanvasElement;
use xterm_js_rs::Terminal;

use crate::tty::TtyMode;

use super::bin::*;
use super::builtins::*;
use super::common::*;
use super::environment::*;
use super::err;
use super::eval::*;
use super::fd::*;
use super::fs::*;
use super::job::*;
use super::pool::ThreadPool as Pool;
use super::reactor::*;
use super::state::*;
use super::stdio::*;
use super::stdout::*;
use super::tty::*;

pub struct Console {
    terminal: Terminal,
    front_buffer: HtmlCanvasElement,
    location: url::Url,
    user_agent: String,
    is_mobile: bool,
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

impl Console {
    pub fn new(
        terminal: Terminal,
        front_buffer: HtmlCanvasElement,
        location: String,
        user_agent: String,
        pool: Pool,
    ) -> Console {
        let mut reactor = Reactor::new();

        let state = Arc::new(Mutex::new(ConsoleState::new()));
        let (stdout, mut tty_rx) = reactor.pipe_out().unwrap();
        let stderr = reactor.dup(&stdout).unwrap();

        let stdout = reactor.fd(stdout);
        let stdout = Stdout::new(stdout);

        let reactor = Arc::new(RwLock::new(reactor));

        // Stdout and Stderr
        {
            let state = state.clone();
            let terminal: Terminal = terminal.clone().dyn_into().unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                while let Some(data) = tty_rx.recv().await {
                    let text = String::from_utf8_lossy(&data[..])[..].replace("\n", "\r\n");
                    terminal.write(text.as_str());

                    let mut state = state.lock().unwrap();
                    state.unfinished_line = is_cleared_line(&text) == false;
                }
            });
        }

        let location = url::Url::parse(&location).unwrap();

        let is_mobile = is_mobile(&user_agent);
        let tty = Tty::new(stdout.clone(), is_mobile);

        let mounts = super::fs::create_root_fs();

        Console {
            terminal,
            front_buffer,
            location,
            is_mobile,
            user_agent,
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

    pub async fn init(&mut self) {
        crate::glue::show_terminal();

        let mut location_file = self
            .mounts
            .new_open_options()
            .create_new(true)
            .write(true)
            .open(Path::new("/etc/location"))
            .unwrap();
        location_file
            .write_all(self.location.as_str().as_bytes())
            .unwrap();

        let run_command = self
            .location
            .query_pairs()
            .filter(|(key, _)| key == "run-command" || key == "init")
            .next()
            .map(|(_, val)| val.to_string());

        if let Some(run_command) = &run_command {
            let mut init_file = self
                .mounts
                .new_open_options()
                .create_new(true)
                .write(true)
                .open(Path::new("/bin/init"))
                .unwrap();
            init_file.write_all(run_command.as_bytes()).unwrap();
        }

        let cols = self.terminal.get_cols();
        let rows = self.terminal.get_rows();
        self.tty.set_bounds(cols, rows).await;

        Console::update_prompt(false, &self.state, &self.tty).await;

        self.tty.draw_welcome().await;
        if run_command.is_some() {
            self.on_data("exec /bin/init".to_string()).await;
            self.on_enter().await;
        } else {
            self.tty.draw_prompt().await;
        }
    }

    pub fn tty(&self) -> &Tty {
        &self.tty
    }

    pub async fn on_enter(&mut self) {
        let mode = self.tty.mode().await;

        self.tty.set_cursor_to_end().await;
        self.tty.draw("\r\n").await;

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

        let reactor = self.reactor.clone();
        let pool = self.pool.clone();
        let (env, last_return, path) = {
            let mut state = self.state.lock().unwrap();
            state.unfinished_line = false;
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
                    self.tty
                        .draw("term: insufficient file handle space\r\n")
                        .await;
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
            console: self.state.clone(),
            stdio,
        };

        let rx = eval(ctx).await;

        let mut tty = self.tty.clone();
        tty.reset_line().await;
        tty.clear_paragraph().await;
        tty.enter_mode(TtyMode::StdIn(job), &self.reactor).await;

        let state = self.state.clone();
        wasm_bindgen_futures::spawn_local(async move {
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
                Ok(EvalPlan::Executed {
                    code,
                    ctx,
                    show_result,
                }) => {
                    debug!("eval executed (code={})", code);
                    let should_line_feed = {
                        let mut state = state.lock().unwrap();
                        state.last_return = code;
                        state.env = ctx.env;
                        state.unfinished_line
                    };
                    if record_history {
                        tty.record_history(cmd).await;
                    }
                    if code != 0 && show_result {
                        let mut chars = String::new();
                        chars += err::exit_code_to_message(code);
                        chars += "\r\n";
                        tty.draw(chars.as_str()).await;
                    } else if should_line_feed {
                        tty.draw("\r\n").await;
                    }
                }
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
                }
                Err(err) => {
                    debug!("eval recv error (err={})", err);
                    tty.draw(format!("term: command failed - {} \r\n", err).as_str())
                        .await;
                }
            };
            tty.reset_line().await;
            Console::update_prompt(multiline_input, &state, &tty).await;
            tty.draw_prompt().await;
        });
    }

    async fn update_prompt(multiline_input: bool, state: &Arc<Mutex<ConsoleState>>, tty: &Tty) {
        let (prompt, prompt_color) = {
            let state = state.lock().unwrap();
            let prompt = state.compute_prompt(multiline_input, false);
            let prompt_color = state.compute_prompt(multiline_input, true);
            (prompt, prompt_color)
        };

        tty.set_prompt(prompt, prompt_color).await;
    }

    pub async fn on_ctrl_l(&mut self) {
        self.tty.reset_line().await;
        self.tty.draw_prompt().await;
        self.terminal.clear();
    }

    pub async fn on_tab(&mut self, _job: Option<Job>) {
        // Later we need to implement auto-complete here when the process is not running
        // and implement spaces when it is
    }

    pub async fn on_page_up(&mut self) {}
    pub async fn on_page_down(&mut self) {}
    pub async fn on_f1(&mut self) {}
    pub async fn on_f2(&mut self) {}
    pub async fn on_f3(&mut self) {}
    pub async fn on_f4(&mut self) {}
    pub async fn on_f5(&mut self) {}
    pub async fn on_f6(&mut self) {}
    pub async fn on_f7(&mut self) {}
    pub async fn on_f8(&mut self) {}
    pub async fn on_f9(&mut self) {}
    pub async fn on_f10(&mut self) {}
    pub async fn on_f11(&mut self) {}
    pub async fn on_f12(&mut self) {}

    pub async fn on_ctrl_c(&mut self, job: Option<Job>) {
        if job.is_none() {
            self.tty.draw("\r\n").await;
        } else {
            self.tty.draw("^C\r\n").await;
        }

        let mode = self.tty.mode().await;
        match mode {
            TtyMode::Null => {}
            TtyMode::Console => {
                self.tty.clear_paragraph().await;
                self.tty.reset_line().await;
                Console::update_prompt(false, &self.state, &self.tty).await;
                self.tty.draw_prompt().await;
            }
            TtyMode::StdIn(job) => {
                {
                    let mut reactor = self.reactor.write().await;
                    reactor.close_job(job, err::ERR_TERMINATED);
                }
                self.tty.enter_mode(TtyMode::Null, &self.reactor).await;
            }
        }
    }

    pub async fn on_resize(&mut self) {
        let cols = self.terminal.get_cols();
        let rows = self.terminal.get_rows();
        self.tty.set_bounds(cols, rows).await;
    }

    pub async fn on_parse(&mut self, data: &str, job: Option<Job>) {
        //error!("on_parse {}", data.as_bytes().iter().map(|byte| format!("\\u{{{:04X}}}", byte).to_owned()).collect::<Vec<String>>().join(""));
        match data {
            "\r" => {
                self.on_enter().await;
            }
            "\u{0003}" => {
                // Ctrl-C
                self.on_ctrl_c(job).await;
            }
            "\u{007F}" => {
                self.tty.backspace().await;
            }
            "\u{0009}" => {
                self.on_tab(job).await;
            }
            "\u{001B}\u{005B}\u{0044}" => {
                self.tty.cursor_left().await;
            }
            "\u{001B}\u{005B}\u{0043}" => {
                self.tty.cursor_right().await;
            }
            "\u{0001}" | "\u{001B}\u{005B}\u{0048}" => {
                self.tty.set_cursor_to_start().await;
            }
            "\u{001B}\u{005B}\u{0046}" => {
                self.tty.set_cursor_to_end().await;
            }
            "\u{001B}\u{005B}\u{0041}" => {
                if job.is_none() {
                    self.tty.cursor_up().await;
                }
            }
            "\u{001B}\u{005B}\u{0042}" => {
                if job.is_none() {
                    self.tty.cursor_down().await;
                }
            }
            "\u{000C}" => {
                self.on_ctrl_l().await;
            }
            "\u{001B}\u{005B}\u{0035}\u{007E}" => {
                self.on_page_up().await;
            }
            "\u{001B}\u{005B}\u{0036}\u{007E}" => {
                self.on_page_down().await;
            }
            "\u{001B}\u{004F}\u{0050}" => self.on_f1().await,
            "\u{001B}\u{004F}\u{0051}" => self.on_f2().await,
            "\u{001B}\u{004F}\u{0052}" => self.on_f3().await,
            "\u{001B}\u{004F}\u{0053}" => self.on_f4().await,
            "\u{001B}\u{005B}\u{0031}\u{0035}\u{007E}" => self.on_f5().await,
            "\u{001B}\u{005B}\u{0031}\u{0037}\u{007E}" => self.on_f6().await,
            "\u{001B}\u{005B}\u{0031}\u{0038}\u{007E}" => self.on_f7().await,
            "\u{001B}\u{005B}\u{0031}\u{0039}\u{007E}" => self.on_f8().await,
            "\u{001B}\u{005B}\u{0032}\u{0030}\u{007E}" => self.on_f9().await,
            "\u{001B}\u{005B}\u{0032}\u{0031}\u{007E}" => self.on_f10().await,
            "\u{001B}\u{005B}\u{0032}\u{0033}\u{007E}" => self.on_f11().await,
            "\u{001B}\u{005B}\u{0032}\u{0034}\u{007E}" => self.on_f12().await,
            data => {
                self.tty.add(data).await;
            }
        }
    }

    pub async fn on_key(
        &mut self,
        _key_code: u32,
        _key: String,
        _alt_key: bool,
        _ctrl_key: bool,
        _meta_key: bool,
    ) {
        // Do nothing for now
    }

    pub async fn on_data(&mut self, mut data: String) {
        let mode = self.tty.mode().await;
        match mode {
            TtyMode::StdIn(job) => {
                // Ctrl-C is not fed to the process and always actioned
                if data == "\u{0003}" {
                    self.on_ctrl_c(Some(job)).await

                // Buffered input will only be sent to the process once a return key is pressed
                // which allows the line to be 'edited' in the terminal before its submitted
                } else if self.tty.is_buffering() {
                    self.on_parse(&data, Some(job)).await

                // When we are sending unbuffered keys the return key is turned into a newline so that its compatible
                // with things like the rpassword crate which simple reads a line of input with a line feed terminator
                // from TTY.
                } else if data == "\r" {
                    data = "\n".to_string();
                    let _ = job.stdin_tx.send(data.into_bytes()).await;

                // Otherwise we just feed the bytes into the STDIN for the process to handle
                } else {
                    let _ = job.stdin_tx.send(data.into_bytes()).await;
                }
            }
            TtyMode::Null => {}
            TtyMode::Console => self.on_parse(&data, None).await,
        }
    }
}
