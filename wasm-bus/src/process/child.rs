use std::io::{self, Read};
use std::future::Future;
use std::task::Context;
use std::task::Poll;
use std::pin::Pin;

use super::*;
use crate::abi::{Call, CallJoin, Recv, call};
use crate::backend::process::*;

/// Representation of a running or exited child process.
///
/// This structure is used to represent and manage child processes. A child
/// process is created via the [`Command`] struct, which configures the
/// spawning process and can itself be constructed using a builder-style
/// interface.
///
/// There is no implementation of [`Drop`] for child processes,
/// so if you do not ensure the `Child` has exited then it will continue to
/// run, even after the `Child` handle to the child process has gone out of
/// scope.
///
/// Calling [`wait`] (or other functions that wrap around it) will make
/// the parent process wait until the child has actually exited before
/// continuing.
///
/// # Examples
///
/// ```should_panic
/// use std::process::Command;
///
/// let mut child = Command::new("/bin/cat")
///                         .arg("file.txt")
///                         .spawn()
///                         .expect("failed to execute child");
///
/// let ecode = child.wait()
///                  .expect("failed to wait on child");
///
/// assert!(ecode.success());
/// ```
///
/// [`wait`]: Child::wait
#[derive(Debug)]
pub struct Child {
    task: Call,

    /// The handle for writing to the child's standard input (stdin), if it has
    /// been captured. To avoid partially moving
    /// the `child` and thus blocking yourself from calling
    /// functions on `child` while using `stdin`,
    /// you might find it helpful:
    ///
    /// ```compile_fail,E0425
    /// let stdin = child.stdin.take().unwrap();
    /// ```
    pub stdin: Option<ChildStdin>,

    /// The handle for reading from the child's standard output (stdout), if it
    /// has been captured. You might find it helpful to do
    ///
    /// ```compile_fail,E0425
    /// let stdout = child.stdout.take().unwrap();
    /// ```
    ///
    /// to avoid partially moving the `child` and thus blocking yourself from calling
    /// functions on `child` while using `stdout`.
    pub stdout: Option<ChildStdout>,
    #[allow(dead_code)]
    stdout_recv: Option<Recv>,

    /// The handle for reading from the child's standard error (stderr), if it
    /// has been captured. You might find it helpful to do
    ///
    /// ```compile_fail,E0425
    /// let stderr = child.stderr.take().unwrap();
    /// ```
    ///
    /// to avoid partially moving the `child` and thus blocking yourself from calling
    /// functions on `child` while using `stderr`.
    pub stderr: Option<ChildStderr>,
    #[allow(dead_code)]
    stderr_recv: Option<Recv>,
}

impl Child {
    // Starts the child process
    pub(super) fn new(
        cmd: &Command,
        stdin_mode: StdioMode,
        stdout_mode: StdioMode,
        stderr_mode: StdioMode,
        pre_open: Vec<String>,
    ) -> Result<Child> {
        let task = call(WAPM_NAME.into(), Spawn {
                path: cmd.path.clone(),
                current_dir: cmd.current_dir.clone(),
                args: cmd.args.clone(),
                stdin_mode,
                stdout_mode,
                stderr_mode,
                pre_open: pre_open.clone(),
            })
            .invoke();

        let (stdout, stdout_recv) = if stdout_mode == StdioMode::Piped {
            let (stdout, tx) = ChildStdout::new();
            let recv = task.recv(move |data: DataStdout| {
                let _ = tx.send(data.0);
            });
            (Some(stdout), Some(recv))
        } else {
            (None, None)
        };

        let (stderr, stderr_recv) = if stderr_mode == StdioMode::Piped {
            let (stderr, tx) = ChildStderr::new();
            let recv = task.recv(move |data: DataStderr| {
                let _ = tx.send(data.0);
            });
            (Some(stderr), Some(recv))
        } else {
            (None, None)
        };

        let stdin = if stdin_mode == StdioMode::Piped {
            let stdin = ChildStdin::new(task.clone());
            Some(stdin)
        } else {
            None
        };

        Ok(Child {
            task,
            stdin,
            stdout,
            stdout_recv,
            stderr,
            stderr_recv,
        })
    }

    /// Forces the child process to exit. If the child has already exited, an [`InvalidInput`]
    /// error is returned.
    ///
    /// The mapping to [`ErrorKind`]s is not part of the compatibility contract of the function,
    /// especially the [`Other`] kind might change to more specific kinds in the future.
    ///
    /// This is equivalent to sending a SIGKILL on Unix platforms.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// let mut command = Command::new("yes");
    /// if let Ok(mut child) = command.spawn() {
    ///     child.kill().expect("command wasn't running");
    /// } else {
    ///     println!("yes command didn't start");
    /// }
    /// ```
    ///
    /// [`ErrorKind`]: io::ErrorKind
    /// [`InvalidInput`]: io::ErrorKind::InvalidInput
    /// [`Other`]: io::ErrorKind::Other
    pub fn kill(&mut self) -> io::Result<()> {
        Ok(self.task.call(OutOfBand::Kill)
            .invoke()
            .join()
            .wait()
            .map_err(|err| err.into_io_error())?)
    }

    /// Returns the OS-assigned process identifier associated with this child.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// let mut command = Command::new("ls");
    /// if let Ok(child) = command.spawn() {
    ///     println!("Child's ID is {}", child.id());
    /// } else {
    ///     println!("ls command didn't start");
    /// }
    /// ```
    pub fn id(&self) -> u32 {
        self.task.id() as u32
    }

    /// Waits for the child to exit completely, returning the status that it
    /// exited with. This function will continue to have the same return value
    /// after it has been called at least once.
    ///
    /// The stdin handle to the child process, if any, will be closed
    /// before waiting. This helps avoid deadlock: it ensures that the
    /// child does not block waiting for input from the parent, while
    /// the parent waits for the child to exit.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// let mut command = Command::new("ls");
    /// if let Ok(mut child) = command.spawn() {
    ///     child.wait().expect("command wasn't running");
    ///     println!("Child has finished its execution!");
    /// } else {
    ///     println!("ls command didn't start");
    /// }
    /// ```
    pub fn wait(self) -> io::Result<ExitStatus> {
        self.task
            .join()
            .wait()
            .map_err(|err| err.into_io_error())
    }

    /// Simultaneously waits for the child to exit and collect all remaining
    /// output on the stdout/stderr handles, returning an `Output`
    /// instance.
    ///
    /// The stdin handle to the child process, if any, will be closed
    /// before waiting. This helps avoid deadlock: it ensures that the
    /// child does not block waiting for input from the parent, while
    /// the parent waits for the child to exit.
    ///
    /// By default, stdin, stdout and stderr are inherited from the parent.
    /// In order to capture the output into this `Result<Output>` it is
    /// necessary to create new pipes between parent and child. Use
    /// `stdout(Stdio::piped())` or `stderr(Stdio::piped())`, respectively.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use wasi_net::{Command, Stdio};
    ///
    /// let child = Command::new("/bin/cat")
    ///     .arg("file.txt")
    ///     .stdout(Stdio::piped())
    ///     .spawn()
    ///     .expect("failed to execute child");
    ///
    /// let output = child
    ///     .wait_with_output()
    ///     .expect("failed to wait on child");
    ///
    /// assert!(output.status.success());
    /// ```
    ///
    pub fn wait_with_output(mut self) -> io::Result<Output> {
        drop(self.stdin.take());

        let taken = (self.stdout.take(), self.stderr.take());
        let status = self.wait()?;

        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        match taken {
            (None, None) => {}
            (Some(mut out), None) => {
                out.read_to_end(&mut stdout).unwrap();
            }
            (None, Some(mut err)) => {
                err.read_to_end(&mut stderr).unwrap();
            }
            (Some(mut out), Some(mut err)) => {
                out.read_to_end(&mut stdout).unwrap();
                err.read_to_end(&mut stderr).unwrap();
            }
        }

        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }

    pub fn join(self) -> ChildJoin
    {
        ChildJoin {
            result: self.task.join()
        }
    }
}

pub struct ChildJoin
{
    result: CallJoin<ExitStatus>
}

impl ChildJoin
{
    /// Attempts to collect the exit status of the child if it has already
    /// exited.
    ///
    /// This function will not block the calling thread and will only
    /// check to see if the child process has exited or not. If the child has
    /// exited then on Unix the process ID is reaped. This function is
    /// guaranteed to repeatedly return a successful exit status so long as the
    /// child has already exited.
    ///
    /// If the child has exited, then `Ok(Some(status))` is returned. If the
    /// exit status is not available at this time then `Ok(None)` is returned.
    /// If an error occurs, then that error is returned.
    ///
    /// Note that unlike `wait`, this function will not attempt to drop stdin.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use wasi_net::Command;
    ///
    /// let mut child = Command::new("ls").spawn().unwrap();
    ///
    /// match child.try_wait() {
    ///     Ok(Some(status)) => println!("exited with: {}", status),
    ///     Ok(None) => {
    ///         println!("status not ready yet, let's really wait");
    ///         let res = child.wait();
    ///         println!("result: {:?}", res);
    ///     }
    ///     Err(e) => println!("error attempting to wait: {}", e),
    /// }
    /// ```
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.result
            .try_wait()
            .map_err(|err| err.into_io_error())
    }
}

/// Its also possible to .await the process
impl Future
for ChildJoin
{
    type Output = io::Result<ExitStatus>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = Pin::new(&mut self.result);
        match result.poll(cx) {
            Poll::Ready(Ok(a)) => Poll::Ready(Ok(a)),
            Poll::Ready(Err(err)) => Poll::Ready(Err(err.into_io_error())),
            Poll::Pending => Poll::Pending
        }
    }
}