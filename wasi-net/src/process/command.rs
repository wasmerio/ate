use std::io;

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
    pub(super) current_dir: Option<String>,
}

impl Command
{
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
            current_dir: None,
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
    pub fn args(&mut self, args: &[&str]) -> &mut Command {
        for arg in args {
            self.args.push(arg.to_string());
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
    ///         .current_dir("/bin")
    ///         .spawn()
    ///         .expect("ls command failed to start");
    /// ```
    ///
    /// [`canonicalize`]: crate::fs::canonicalize
    pub fn current_dir(&mut self, dir: &str) -> &mut Command {
        self.current_dir = Some(dir.to_string());
        self
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
        Child::new(self)
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