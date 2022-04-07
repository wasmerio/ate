#![allow(unused_imports)]
#![allow(dead_code)]
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
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
use super::wizard_executor::*;
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
    compiler: Compiler,
    abi: Arc<dyn ConsoleAbi>,
    wizard: Option<WizardExecutor>,
    whitelabel: bool,
    bootstrap_token: Option<String>,
    no_welcome: bool,
}

impl Drop for Console {
    fn drop(&mut self) {
        let state = self.state.clone();
        let reactor = self.reactor.clone();
        let system = System::default();
        let work = async move {
            reactor.write().await.clear();
            state.lock().unwrap().clear_mounts();
        };
        system.fork_shared(move || work);
    }
}

pub enum StdioEvent {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    Log(Vec<u8>),
}

impl Console {
    pub fn new(
        location: String,
        user_agent: String,
        compiler: Compiler,
        abi: Arc<dyn ConsoleAbi>,
        wizard: Option<Box<dyn WizardAbi + Send + Sync + 'static>>,
        fs: UnionFileSystem,
        #[cfg(feature = "cached_compiling")] compiled_modules: Arc<CachedCompiledModules>,
    ) -> Console {
        let bins = BinFactory::new(
            #[cfg(feature = "cached_compiling")]
            compiled_modules,
        );
        let reactor = Arc::new(RwLock::new(Reactor::new()));

        Self::new_ext(
            location, user_agent, compiler, abi, wizard, fs, bins, reactor,
        )
    }

    pub fn new_ext(
        location: String,
        user_agent: String,
        compiler: Compiler,
        abi: Arc<dyn ConsoleAbi>,
        wizard: Option<Box<dyn WizardAbi + Send + Sync + 'static>>,
        fs: UnionFileSystem,
        bins: BinFactory,
        reactor: Arc<RwLock<Reactor>>,
    ) -> Console {
        let location = url::Url::parse(&location).unwrap();
        let is_mobile = is_mobile(&user_agent);

        let unfinished_line = Arc::new(AtomicBool::new(false));
        let mut state = ConsoleState::new(fs, unfinished_line.clone());
        if let Some(origin) = location.domain().clone() {
            state.env.set_var("ORIGIN", origin.to_string());
        }
        state.env.set_var("LOCATION", location.to_string());

        let state = Arc::new(Mutex::new(state));
        let tty = Tty::channel(&abi, &unfinished_line, is_mobile);

        let exec_factory = EvalFactory::new(
            bins.clone(),
            tty.clone(),
            reactor.clone(),
            tty.stdout(),
            tty.stderr(),
            tty.log(),
        );

        let wizard = wizard.map(|a| WizardExecutor::new(a));

        let mut ret = Console {
            location,
            is_mobile,
            user_agent,
            bins,
            state,
            stdout: tty.stdout(),
            stderr: tty.stderr(),
            tty,
            reactor,
            exec: exec_factory,
            compiler,
            abi,
            wizard,
            whitelabel: false,
            bootstrap_token: None,
            no_welcome: false,
        };

        ret.new_init();

        ret
    }

    fn new_init(&mut self) {
        self.whitelabel = self.location.query_pairs().any(|(key, _)| key == "wl");

        if let Some(prompt) = self
            .location
            .query_pairs()
            .filter(|(key, _)| key == "prompt")
            .next()
            .map(|(_, val)| val.to_string())
        {
            self.state.lock().unwrap().user = prompt;
        }

        let no_welcome = self
            .location
            .query_pairs()
            .any(|(key, _)| key == "no_welcome" || key == "no-welcome");

        let token = self
            .location
            .query_pairs()
            .filter(|(key, _)| key == "token")
            .next()
            .map(|(_, val)| val.to_string());

        self.bootstrap_token = token;
        self.no_welcome = no_welcome;
    }

    pub async fn prepare(&mut self) {
        let rect = self.abi.console_rect().await;
        self.tty.set_bounds(rect.cols, rect.rows).await;

        Console::update_prompt(false, &self.state, &self.tty).await;
    }

    pub async fn init(&mut self, run_command: Option<String>) {
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

        self.prepare().await;

        if self.wizard.is_some() {
            self.on_wizard(None).await;
        } else {
            self.start_shell().await;
        }
    }

    pub async fn start_shell(&mut self) {
        if self.whitelabel == false && self.no_welcome == false {
            self.tty.draw_welcome().await;
        }

        let has_init = self
            .state
            .lock()
            .unwrap()
            .rootfs
            .metadata(&Path::new("/bin/init"))
            .is_ok();

        if let Some(token) = self.bootstrap_token.take() {
            let cmd = if has_init {
                format!("login --token {} && source /bin/init", token)
            } else {
                format!("login --token {}", token)
            };
            self.on_enter_internal(cmd, false).await;
        } else if has_init {
            self.on_enter_internal("source /bin/init".to_string(), false)
                .await;
        } else {
            self.tty.draw_prompt().await;
        }
    }

    pub fn tty(&self) -> &Tty {
        &self.tty
    }

    pub fn tty_mut(&mut self) -> &mut Tty {
        &mut self.tty
    }

    pub fn abi(&self) -> Arc<dyn ConsoleAbi> {
        self.abi.clone()
    }

    pub fn reactor(&self) -> Arc<RwLock<Reactor>> {
        self.reactor.clone()
    }

    pub fn stdout_fd(&self) -> Fd {
        self.stdout.fd()
    }

    pub fn stderr_fd(&self) -> Fd {
        self.stderr.clone()
    }

    pub fn exec_factory(&self) -> EvalFactory {
        self.exec.clone()
    }

    pub async fn new_job(&mut self) -> Option<Job> {
        // Generate the job and make it the active version
        let job = {
            let mut reactor = self.reactor.write().await;
            let job = match reactor.generate_job() {
                Ok((_, job)) => job,
                Err(_) => {
                    drop(reactor);
                    self.tty.draw("term: insufficient job space\r\n").await;
                    self.tty.reset_line().await;
                    self.tty.draw_prompt().await;
                    return None;
                }
            };
            reactor.set_current_job(job.id);
            job
        };
        Some(job)
    }

    pub fn root_fs(&self) -> UnionFileSystem {
        let state = self.state.lock().unwrap();
        state.rootfs.clone()
    }

    pub fn new_spawn_context(&self, job: &Job) -> SpawnContext {
        let ctx = {
            let state = self.state.lock().unwrap();
            SpawnContext::new(
                self.abi.clone(),
                state.env.clone(),
                job.clone(),
                job.stdin.clone(),
                self.stdout.fd.clone(),
                self.stderr.clone(),
                false,
                state.path.clone(),
                Vec::new(),
                state.rootfs.clone(),
                self.compiler,
            )
        };
        ctx
    }

    pub async fn on_wizard(&mut self, cmd: Option<String>) {
        self.tty.reset_paragraph().await;
        self.tty.reset_line().await;
        self.tty.set_echo(true).await;

        if let Some(wizard) = self.wizard.as_mut() {
            match wizard.feed(&self.abi, cmd).await {
                WizardExecutorAction::More { echo } => {
                    self.tty.set_echo(echo).await;
                    return;
                }
                WizardExecutorAction::Done => {
                    drop(wizard);

                    if let Some(wizard) = self.wizard.take() {
                        if let Some(token) = wizard.token() {
                            self.bootstrap_token = Some(token);
                        }
                    }

                    self.start_shell().await;
                    return;
                }
            }
        }
    }

    pub async fn on_enter(&mut self) {
        self.tty.set_cursor_to_end().await;
        let cmd = self.tty.get_paragraph().await;

        if self.wizard.is_some() {
            self.on_wizard(Some(cmd)).await;
            return;
        }

        self.tty.draw("\r\n").await;

        self.on_enter_internal(cmd, true).await
    }

    pub async fn on_enter_internal(&mut self, mut cmd: String, record_history: bool) {
        let mode = self.tty.mode().await;
        if let TtyMode::StdIn(job) = mode {
            cmd += "\n";
            self.tty.reset_line().await;
            self.tty.reset_paragraph().await;
            //error!("on_stdin {}", cmd.as_bytes().iter().map(|byte| format!("\\u{{{:04X}}}", byte).to_owned()).collect::<Vec<String>>().join(""));
            let _ = job
                .stdin_tx
                .send(FdMsg::new(cmd.into_bytes(), FdFlag::Stdin(true)))
                .await;
            return;
        }

        if cmd.len() <= 0 {
            self.tty.reset_line().await;
            self.tty.draw_prompt().await;
            return;
        }

        // Generate the job and make it the active version
        let job = if let Some(j) = self.new_job().await {
            j
        } else {
            return;
        };

        // Switch the console to this particular job
        let mut tty = self.tty.clone();
        tty.reset_line().await;
        tty.reset_paragraph().await;
        tty.enter_mode(TtyMode::StdIn(job.clone()), &self.reactor)
            .await;

        // Spawn the process and attach it to the job
        let ctx = self.new_spawn_context(&job);

        // Spawn a background thread that will process the result
        // of the process that we just started
        let exec = self.exec.clone();
        let reactor = self.reactor.clone();
        let system = System::default();
        let state = self.state.clone();
        let mut stdout = ctx.stdout.clone();
        let mut stderr = ctx.stderr.clone();
        system.fork_dedicated(move || {
            let mut process = exec.eval(cmd.clone(), ctx);
            async move {
                // Wait for the process to finish
                let rx = process.recv().await;
                drop(process);

                // Flush all the pipes
                let _ = stdout.flush_async().await;
                let _ = stderr.flush_async().await;
                let _ = tty.flush_async().await;

                // Switch back to console mode
                tty.reset_line().await;
                tty.reset_paragraph().await;
                tty.enter_mode(TtyMode::Console, &reactor).await;
                tty.reset_history_cursor().await;

                // Process the result
                let mut multiline_input = false;
                if let Some(rx) = rx {
                    match rx.status {
                        EvalStatus::Executed { code, show_result } => {
                            debug!("eval executed (code={})", code);
                            let should_line_feed = {
                                let state = state.lock().unwrap();
                                state
                                    .unfinished_line
                                    .load(std::sync::atomic::Ordering::Acquire)
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
                        EvalStatus::InternalError => {
                            debug!("eval internal error");
                            tty.draw("term: internal error\r\n").await;
                        }
                        EvalStatus::MoreInput => {
                            debug!("eval more input");
                            multiline_input = true;
                            tty.add(cmd.as_str()).await;
                        }
                        EvalStatus::Invalid => {
                            debug!("eval invalid");
                            tty.draw("term: invalid command\r\n").await;
                        }
                    }

                    // Process any changes to the global state
                    {
                        let ctx: EvalContext = rx.ctx;
                        let mut state = state.lock().unwrap();
                        state.rootfs = ctx.root.sanitize();
                        state.env = ctx.env;
                        state.path = ctx.working_dir;
                        state.last_return = ctx.last_return;
                    }
                } else {
                    debug!("eval recv erro");
                    tty.draw(format!("term: command failed\r\n").as_str()).await;
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
        if self.wizard.is_some() {
            return;
        }

        self.tty.reset_line().await;
        self.tty.draw_prompt().await;
        self.abi.cls().await;
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
        if self.wizard.is_some() {
            self.abi.exit().await;
            return;
        }

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
                    reactor.close_job(job, std::num::NonZeroU32::new(err::ERR_TERMINATED).unwrap());
                }
                self.tty.enter_mode(TtyMode::Null, &self.reactor).await;
            }
        }
    }

    pub async fn on_resize(&mut self) {
        let rect = self.abi.console_rect().await;
        self.tty.set_bounds(rect.cols, rect.rows).await;
    }

    pub async fn on_parse(&mut self, data: &str, job: Option<Job>) {
        //error!("on_parse {}", data.as_bytes().iter().map(|byte| format!("\\u{{{:04X}}}", byte).to_owned()).collect::<Vec<String>>().join(""));
        match data {
            "\r" | "\u{000A}" => {
                self.on_enter().await;
            }
            "\u{0003}" => {
                // Ctrl-C
                self.on_ctrl_c(job).await;
            }
            "\u{007F}" => {
                self.tty.backspace().await;
            }
            "\u{0009}" if self.wizard.is_none() => {
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
            "\u{001B}\u{005B}\u{0041}" if self.wizard.is_none() => {
                if job.is_none() {
                    self.tty.cursor_up().await;
                }
            }
            "\u{001B}\u{005B}\u{0042}" if self.wizard.is_none() => {
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
                // Buffered input will only be sent to the process once a return key is pressed
                // which allows the line to be 'edited' in the terminal before its submitted
                if self.tty.is_buffering() {
                    // Ctrl-C is not fed to the process and always actioned
                    if data == "\u{0003}" {
                        self.on_ctrl_c(Some(job)).await
                    } else {
                        self.on_parse(&data, Some(job)).await
                    }

                // When we are sending unbuffered keys the return key is turned into a newline so that its compatible
                // with things like the rpassword crate which simple reads a line of input with a line feed terminator
                // from TTY.
                } else if data == "\r" || data == "\u{000A}" {
                    data = "\n".to_string();
                    let _ = job
                        .stdin_tx
                        .send(FdMsg::new(data.into_bytes(), FdFlag::Stdin(true)))
                        .await;

                // Otherwise we just feed the bytes into the STDIN for the process to handle
                } else {
                    let _ = job
                        .stdin_tx
                        .send(FdMsg::new(data.into_bytes(), FdFlag::Stdin(true)))
                        .await;
                }
            }
            TtyMode::Null => {}
            TtyMode::Console => self.on_parse(&data, None).await,
        }
    }
}
