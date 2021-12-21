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

use crate::tty::TtyMode;

use super::bin_factory::*;
use super::builtins::*;
use super::common::*;
use super::environment::*;
use super::err;
use super::eval::*;
use super::fd::*;
use super::fs::*;
use super::job::*;
use super::pipe::*;
use super::reactor::*;
use super::state::*;
use super::stdio::*;
use super::stdout::*;
use super::tty::*;
use crate::api::*;
use crate::wasmer_vfs::FileSystem;

pub struct Console {
    location: url::Url,
    user_agent: String,
    is_mobile: bool,
    state: Arc<Mutex<ConsoleState>>,
    bins: BinFactory,
    tty: Tty,
    reactor: Arc<RwLock<Reactor>>,
    stdout: Stdout,
    stderr: Fd,
    exec: EvalFactory,
}

impl Console {
    pub fn new(location: String, user_agent: String) -> Console {
        let reactor = Reactor::new();

        let state = Arc::new(Mutex::new(ConsoleState::new(super::fs::create_root_fs())));
        let (stdout, mut tty_rx) = pipe_out(true);
        let (log, mut log_rx) = pipe_out(false);
        let stderr = stdout.clone();
        let stdout = Stdout::new(stdout);

        let system = System::default();
        let reactor = Arc::new(RwLock::new(reactor));

        // Stdout and Stderr
        {
            let state = state.clone();
            system.fork_local(async move {
                while let Some(data) = tty_rx.recv().await {
                    let text = String::from_utf8_lossy(&data[..])[..].replace("\n", "\r\n");
                    let is_unfinished = is_cleared_line(&text) == false;
                    system.print(text).await;

                    let mut state = state.lock().unwrap();
                    state.unfinished_line = is_unfinished;
                }
            });
        }

        // Log
        system.fork_local(async move {
            while let Some(data) = log_rx.recv().await {
                let txt = String::from_utf8_lossy(&data[..]);
                let mut txt = txt.as_ref();
                while txt.ends_with("\n") || txt.ends_with("\r") {
                    txt = &txt[..(txt.len() - 1)];
                }
                system.log(txt.to_string()).await;
            }
        });

        let location = url::Url::parse(&location).unwrap();

        let is_mobile = is_mobile(&user_agent);
        let tty = Tty::new(stdout.clone(), is_mobile);

        let bins = BinFactory::new();
        let exec_factory = EvalFactory::new(
            bins.clone(),
            tty.clone(),
            reactor.clone(),
            stdout.clone(),
            stderr.clone(),
            log.clone(),
        );

        Console {
            location,
            is_mobile,
            user_agent,
            bins,
            state,
            stdout,
            stderr,
            tty,
            reactor,
            exec: exec_factory,
        }
    }

    pub async fn init(&mut self) {
        let mut location_file = self
            .state
            .lock()
            .unwrap()
            .rootfs
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
                .state
                .lock()
                .unwrap()
                .rootfs
                .new_open_options()
                .create_new(true)
                .write(true)
                .open(Path::new("/bin/init"))
                .unwrap();
            init_file.write_all(run_command.as_bytes()).unwrap();
        }

        let system = System::default();
        let rect = system.console_rect().await;
        self.tty.set_bounds(rect.cols, rect.rows).await;

        Console::update_prompt(false, &self.state, &self.tty).await;

        self.tty.draw_welcome().await;
        if run_command.is_some() {
            self.on_data("source /bin/init".to_string()).await;
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
            self.tty.reset_paragraph().await;
            let _ = job.stdin_tx.send(cmd.into_bytes()).await;
            return;
        }

        if cmd.len() <= 0 {
            self.tty.reset_line().await;
            self.tty.draw_prompt().await;
            return;
        }

        let reactor = self.reactor.clone();
        let (env, path, root) = {
            let mut state = self.state.lock().unwrap();
            state.unfinished_line = false;
            let env = state.env.clone();
            let path = state.path.clone();
            let root = state.rootfs.clone();
            (env, path, root)
        };

        // Generate the job and make it the active version
        let job = {
            let mut reactor = reactor.write().await;
            let job = match reactor.generate_job(path.clone(), env.clone(), root.clone()) {
                Ok((_, job)) => job,
                Err(_) => {
                    drop(reactor);
                    self.tty.draw("term: insufficient job space\r\n").await;
                    self.tty.reset_line().await;
                    self.tty.draw_prompt().await;
                    return;
                }
            };
            reactor.set_current_job(job.id);
            job
        };

        // Switch the console to this particular job
        let mut tty = self.tty.clone();
        tty.reset_line().await;
        tty.reset_paragraph().await;
        tty.enter_mode(TtyMode::StdIn(job.clone()), &self.reactor)
            .await;

        // Spawn the process and attach it to the job
        let ctx = SpawnContext::new(
            cmd.clone(),
            env,
            job.clone(),
            job.stdin.clone(),
            self.stdout.fd.clone(),
            self.stderr.clone(),
            false,
            job.working_dir.clone(),
            Vec::new(),
            root,
        );

        // Spawn a background thread that will process the result
        // of the process that we just started
        let exec = self.exec.clone();
        let system = System::default();
        let state = self.state.clone();
        system.fork_dedicated(move || {
            let mut process = exec.eval(ctx);
            async move {
                // Wait for the process to finish
                let rx = process.recv().await;
                drop(process);

                // Switch back to console mode
                tty.reset_line().await;
                tty.reset_paragraph().await;
                tty.enter_mode(TtyMode::Console, &reactor).await;
                let record_history = if let Some(history) = tty.get_selected_history().await {
                    history != cmd
                } else {
                    true
                };
                tty.reset_history_cursor().await;

                // Process the result
                let mut multiline_input = false;
                match rx {
                    Some(EvalPlan::Executed {
                        code,
                        mut ctx,
                        show_result,
                    }) => {
                        debug!("eval executed (code={})", code);
                        let should_line_feed = {
                            let mut state = state.lock().unwrap();
                            state.rootfs.mounts.append(&mut ctx.new_mounts);
                            state.last_return = code;
                            if let Some(path) = ctx.new_pwd {
                                state.path = path;
                            }
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
                    Some(EvalPlan::InternalError) => {
                        debug!("eval internal error");
                        tty.draw("term: internal error\r\n").await;
                    }
                    Some(EvalPlan::MoreInput) => {
                        debug!("eval more input");
                        multiline_input = true;
                        tty.add(cmd.as_str()).await;
                    }
                    Some(EvalPlan::Invalid) => {
                        debug!("eval invalid");
                        tty.draw("term: invalid command\r\n").await;
                    }
                    None => {
                        debug!("eval recv erro");
                        tty.draw(format!("term: command failed\r\n").as_str()).await;
                    }
                }

                // Now draw the prompt ready for the next
                tty.reset_line().await;
                Console::update_prompt(multiline_input, &state, &tty).await;
                tty.draw_prompt().await;
            }
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
        System::default().cls().await;
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
                self.tty.reset_paragraph().await;
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
        let rect = System::default().console_rect().await;
        self.tty.set_bounds(rect.cols, rect.rows).await;
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
