use std::{ffi::OsStr, io};

use super::*;

/// A process builder, providing fine-grained control
/// over how a new process should be spawned.
///
/// A default configuration can be
/// generated using `Command::new(program)`, where `program` gives a path to the
/// program to be executed. Additional builder methods allow the configuration
/// to be changed (for example, by adding arguments) prior to spawning:
///
/// ```
/// use wasi_net::Command;
///
/// let output = if cfg!(target_os = "windows") {
///     Command::new("cmd")
///             .args(&["/C", "echo hello"])
///             .output()
///             .expect("failed to execute process")
/// } else {
///     Command::new("sh")
///             .arg("-c")
///             .arg("echo hello")
///             .output()
///             .expect("failed to execute process")
/// };
///
/// let hello = output.stdout;
/// ```
///
/// `Command` can be reused to spawn multiple processes. The builder methods
/// change the command without needing to immediately spawn the process.
///
/// ```no_run
/// use wasi_net::Command;
///
/// let mut echo_hello = Command::new("sh");
/// echo_hello.arg("-c")
///           .arg("echo hello");
/// let hello_1 = echo_hello.output().expect("failed to execute process");
/// let hello_2 = echo_hello.output().expect("failed to execute process");
/// ```
///
/// Similarly, you can call builder methods after spawning a process and then
/// spawn a new process with the modified settings.
///
/// ```no_run
/// use wasi_net::Command;
///
/// let mut list_dir = Command::new("ls");
///
/// // Execute `ls` in the current directory of the program.
/// list_dir.status().expect("process failed to execute");
/// ```
#[derive(Debug, Clone)]
pub struct Command {
    pub(super) path: String,
    pub(super) args: Vec<String>,
    pub(super) chroot: bool,
    pub(super) working_dir: Option<String>,
    pub(super) stdin: Option<Stdio>,
    pub(super) stdout: Option<Stdio>,
    pub(super) stderr: Option<Stdio>,
    pub(super) pre_open: Vec<String>,
    pub(super) instance: Option<CommandInstance>,
}

#[derive(Debug, Clone)]
pub struct CommandInstance
{
    pub instance: String,
    pub access_token: String,
}

impl Command {
    /// Constructs a new `Command` for launching the program at
    /// path `program`, with the following default configuration:
    ///
    /// * No arguments to the program
    /// * Inherit the current process's environment
    /// * Inherit the current process's working directory
    /// * Inherit stdin/stdout/stderr for `spawn` or `status`, but create pipes for `output`
    ///
    /// Builder methods are provided to change these defaults and
    /// otherwise configure the process.
    ///
    /// If `program` is not an absolute path, the `PATH` will be searched in
    /// an OS-defined way.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// Command::new("sh")
    ///         .spawn()
    ///         .expect("sh command failed to start");
    /// ```
    pub fn new(path: &str) -> Command {
        Command {
            path: path.to_string(),
            args: Vec::new(),
            chroot: false,
            working_dir: None,
            stdin: None,
            stdout: None,
            stderr: None,
            pre_open: Vec::new(),
            instance: None,
        }
    }

    /// Adds an argument to pass to the program.
    ///
    /// Only one argument can be passed per use. So instead of:
    ///
    /// ```no_run
    /// # wasi_net::Command::new("sh")
    /// .arg("-C /path/to/repo")
    /// # ;
    /// ```
    ///
    /// usage would be:
    ///
    /// ```no_run
    /// # wasi_net::Command::new("sh")
    /// .arg("-C")
    /// .arg("/path/to/repo")
    /// # ;
    /// ```
    ///
    /// To pass multiple arguments see [`args`].
    ///
    /// [`args`]: Command::args
    ///
    /// Note that the argument is not passed through a shell, but given
    /// literally to the program. This means that shell syntax like quotes,
    /// escaped characters, word splitting, glob patterns, substitution, etc.
    /// have no effect.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// Command::new("ls")
    ///         .arg("-l")
    ///         .arg("-a")
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    pub fn arg(&mut self, arg: &str) -> &mut Command {
        self.args.push(arg.to_string());
        self
    }

    /// Adds multiple arguments to pass to the program.
    ///
    /// To pass a single argument see [`arg`].
    ///
    /// [`arg`]: Command::arg
    ///
    /// Note that the arguments are not passed through a shell, but given
    /// literally to the program. This means that shell syntax like quotes,
    /// escaped characters, word splitting, glob patterns, substitution, etc.
    /// have no effect.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// Command::new("ls")
    ///         .args(&["-l", "-a"])
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.args.push(arg.as_ref().to_string_lossy().into_owned());
        }
        self
    }

    /// Sets the working directory for the child process.
    ///
    /// # Platform-specific behavior
    ///
    /// If the program path is relative (e.g., `"./script.sh"`), it's ambiguous
    /// whether it should be interpreted relative to the parent's working
    /// directory or relative to `current_dir`. The behavior in this case is
    /// platform specific and unstable, and it's recommended to use
    /// [`canonicalize`] to get an absolute program path instead.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// Command::new("ls")
    ///         .working_dir("/bin")
    ///         .chroot(true)
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    ///
    /// [`canonicalize`]: crate::fs::canonicalize
    pub fn working_dir(&mut self, dir: &str) -> &mut Command {
        self.working_dir = Some(dir.to_string());
        self
    }

    /// Indicates that the root file system will be pivoted to a particular
    /// path as if it were the root.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// Command::new("ls")
    ///         .working_dir("/bin")
    ///         .chroot(true)
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    pub fn chroot(&mut self, chroot: bool) -> &mut Command {
        self.chroot = chroot;
        self
    }

    /// Runs the process on a server side session
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// Command::new("ls")
    ///         .session("my-session", "71248123")
    ///         .working_dir("/bin")
    ///         .chroot(true)
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    pub fn instance(&mut self, instance: &str, access_token: &str) -> &mut Command {
        self.instance = Some(CommandInstance {
            instance: instance.to_string(),
            access_token: access_token.to_string()
        });
        self
    }

    /// Configuration for the child process's standard input (stdin) handle.
    ///
    /// Defaults to [`inherit`] when used with `spawn` or `status`, and
    /// defaults to [`piped`] when used with `output`.
    ///
    /// [`inherit`]: Stdio::inherit
    /// [`piped`]: Stdio::piped
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::{Command, Stdio};
    ///
    /// Command::new("ls")
    ///         .stdin(Stdio::null())
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    pub fn stdin(&mut self, cfg: Stdio) -> &mut Command {
        self.stdin = Some(cfg);
        self
    }

    /// Configuration for the child process's standard output (stdout) handle.
    ///
    /// Defaults to [`inherit`] when used with `spawn` or `status`, and
    /// defaults to [`piped`] when used with `output`.
    ///
    /// [`inherit`]: Stdio::inherit
    /// [`piped`]: Stdio::piped
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use std::process::{Command, Stdio};
    ///
    /// Command::new("ls")
    ///         .stdout(Stdio::null())
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    pub fn stdout(&mut self, cfg: Stdio) -> &mut Command {
        self.stdout = Some(cfg);
        self
    }

    /// Configuration for the child process's standard error (stderr) handle.
    ///
    /// Defaults to [`inherit`] when used with `spawn` or `status`, and
    /// defaults to [`piped`] when used with `output`.
    ///
    /// [`inherit`]: Stdio::inherit
    /// [`piped`]: Stdio::piped
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use std::process::{Command, Stdio};
    ///
    /// Command::new("ls")
    ///         .stderr(Stdio::null())
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    pub fn stderr(&mut self, cfg: Stdio) -> &mut Command {
        self.stderr = Some(cfg);
        self
    }

    /// Pre-opens a directory before launching the process
    pub fn pre_open(&mut self, path: String) -> &mut Command {
        self.pre_open.push(path);
        self
    }

    async fn prep(&self) -> io::Result<Child> {
        let stdin = self
            .stdin
            .as_ref()
            .map(|a| a.clone())
            .unwrap_or(Stdio::inherit());
        let stdout = self
            .stdout
            .as_ref()
            .map(|a| a.clone())
            .unwrap_or(Stdio::inherit());
        let stderr = self
            .stderr
            .as_ref()
            .map(|a| a.clone())
            .unwrap_or(Stdio::inherit());
        let preopen = self.pre_open.clone();
        let instance = self.instance.clone()
            .map(|a| {
                super::child::ChildInstance {
                    instance: a.instance,
                    access_token: a.access_token
                }
            });
        Child::new(self, instance, stdin.mode, stdout.mode, stderr.mode, preopen).await
    }

    /// Executes the command as a child process, returning a handle to it.
    ///
    /// By default, stdin, stdout and stderr are inherited from the parent.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// Command::new("ls")
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    pub fn spawn(&self) -> io::Result<Child> {
        wasm_bus::task::block_on(self.prep())
    }

    /// Executes the command as a child process to completion asynchronously
    ///
    /// By default, stdin, stdout and stderr are inherited from the parent.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// Command::new("ls")
    ///         .execute()
    ///         .await
    ///         .expect("ls command failed to start");
    /// ```
    pub async fn execute(&self) -> io::Result<ExitStatus> {
        Ok(self.prep().await?.join().await?)
    }

    /// Executes the command as a child process, waiting for it to finish and
    /// collecting all of its output.
    ///
    /// By default, stdout and stderr are captured (and used to provide the
    /// resulting output). Stdin is not inherited from the parent and any
    /// attempt by the child process to read from the stdin stream will result
    /// in the stream immediately closing.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use wasi_net::Command;
    /// use std::io::{self, Write};
    /// let output = Command::new("/bin/cat")
    ///                      .arg("file.txt")
    ///                      .output()
    ///                      .expect("failed to execute process");
    ///
    /// println!("status: {}", output.status);
    /// io::stdout().write_all(&output.stdout).unwrap();
    /// io::stderr().write_all(&output.stderr).unwrap();
    ///
    /// assert!(output.status.success());
    /// ```
    pub fn output(&mut self) -> io::Result<Output> {
        if self.stdin.is_none() {
            self.stdin = Some(Stdio::null());
        }
        if self.stdout.is_none() {
            self.stdout = Some(Stdio::piped());
        }
        if self.stderr.is_none() {
            self.stderr = Some(Stdio::piped());
        }
        Ok(self.spawn()?.wait_with_output()?)
    }

    /// Executes a command as a child process, waiting for it to finish and
    /// collecting its status.
    ///
    /// By default, stdin, stdout and stderr are inherited from the parent.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use wasi_net::Command;
    ///
    /// let status = Command::new("/bin/cat")
    ///                      .arg("file.txt")
    ///                      .status()
    ///                      .expect("failed to execute process");
    ///
    /// println!("process finished with: {}", status);
    ///
    /// assert!(status.success());
    /// ```
    pub fn status(&mut self) -> io::Result<ExitStatus> {
        Ok(self.spawn()?.wait()?)
    }
}
