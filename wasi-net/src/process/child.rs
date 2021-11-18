use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::fs::File;
use std::io::{self, Read, Write};

use crate::backend::utils::*;
use crate::backend::Command as BackendCommand;
use crate::backend::Response as BackendResponse;
use crate::backend::MessageProcess;
use super::*;

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
    pid: u32,
    cmd: Command,
    worker: Arc<Mutex<Worker>>,
    rx_exit: mpsc::Receiver<ExitStatus>,

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
}

impl Child
{
    // Starts the child process
    pub(super) fn new(cmd: &Command) -> Result<Child>
    {
        let submit = BackendCommand::SpawnProcessVersion1 {
            path: cmd.path.clone(),
            current_dir: cmd.current_dir.clone(),
            args: cmd.args.clone(),
        };
        let mut submit = submit.serialize()?;
        submit += "\n";

        let mut file = File::open("/dev/process")?;

        let _ = file.write_all(submit.as_bytes());

        let res = read_response(&mut file)?;
        let pid = match res {
            BackendResponse::SpawnedProcessVersion1 { pid } => {
                pid
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "the socket does not support this response type",
                ));
            }
        };

        let (worker, stdin, stdout, stderr, rx_exit)
            = Worker::new(file);

        Ok(Child {
            pid,
            worker,
            cmd: cmd.clone(),
            stdin: Some(stdin),
            stdout: Some(stdout),
            stderr: Some(stderr),
            rx_exit,
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
        self.worker.lock().unwrap().send(MessageProcess::Kill)
            .or_else(|_| Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "failed to notify the process to kill itself",
            )))?;
        Ok(())
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
        self.pid
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
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        loop {
            let has_exited = self.worker.lock().unwrap().work() == false;
            match self.rx_exit.try_recv() {
                Ok(exitcode) => {
                    return Ok(exitcode);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "the process worker already exited",
                    ));
                }
                _ => { }
            }
            if has_exited {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "the process worker already exited",
                ));
            }
        }
    }

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
        match self.rx_exit.try_recv() {
            Ok(exitcode) => {
                Ok(Some(exitcode))
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "the process worker exited",
                ))
            }
            Err(mpsc::TryRecvError::Empty) => {
                Ok(None)
            }
        }
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

        let status = self.wait()?;

        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        match (self.stdout.take(), self.stderr.take()) {
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

        Ok(Output { status, stdout, stderr })
    }
}