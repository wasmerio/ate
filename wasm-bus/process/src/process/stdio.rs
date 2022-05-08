use crate::api::*;

/// Describes what to do with a standard I/O stream for a child process when
/// passed to the [`stdin`], [`stdout`], and [`stderr`] methods of [`Command`].
///
/// [`stdin`]: Command::stdin
/// [`stdout`]: Command::stdout
/// [`stderr`]: Command::stderr
#[derive(Debug, Clone)]
pub struct Stdio {
    pub(super) mode: StdioMode,
}

impl Stdio {
    /// A new pipe should be arranged to connect the parent and child processes.
    ///
    /// # Examples
    ///
    /// With stdout:
    ///
    /// ```no_run
    /// use std::process::{Command, Stdio};
    ///
    /// let output = Command::new("echo")
    ///     .arg("Hello, world!")
    ///     .stdout(Stdio::piped())
    ///     .output()
    ///     .expect("Failed to execute command");
    ///
    /// assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, world!\n");
    /// // Nothing echoed to console
    /// ```
    ///
    /// With stdin:
    ///
    /// ```no_run
    /// use std::io::Write;
    /// use std::process::{Command, Stdio};
    ///
    /// let mut child = Command::new("rev")
    ///     .stdin(Stdio::piped())
    ///     .stdout(Stdio::piped())
    ///     .spawn()
    ///     .expect("Failed to spawn child process");
    ///
    /// let mut stdin = child.stdin.take().expect("Failed to open stdin");
    /// std::thread::spawn(move || {
    ///     stdin.write_all("Hello, world!".as_bytes()).expect("Failed to write to stdin");
    /// });
    ///
    /// let output = child.wait_with_output().expect("Failed to read stdout");
    /// assert_eq!(String::from_utf8_lossy(&output.stdout), "!dlrow ,olleH");
    /// ```
    ///
    /// Writing more than a pipe buffer's worth of input to stdin without also reading
    /// stdout and stderr at the same time may cause a deadlock.
    /// This is an issue when running any program that doesn't guarantee that it reads
    /// its entire stdin before writing more than a pipe buffer's worth of output.
    /// The size of a pipe buffer varies on different targets.
    ///
    pub fn piped() -> Stdio {
        Stdio {
            mode: StdioMode::Piped,
        }
    }

    /// The child inherits from the corresponding parent descriptor.
    ///
    /// # Examples
    ///
    /// With stdout:
    ///
    /// ```no_run
    /// use std::process::{Command, Stdio};
    ///
    /// let output = Command::new("echo")
    ///     .arg("Hello, world!")
    ///     .stdout(Stdio::inherit())
    ///     .output()
    ///     .expect("Failed to execute command");
    ///
    /// assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    /// // "Hello, world!" echoed to console
    /// ```
    ///
    /// With stdin:
    ///
    /// ```no_run
    /// use std::process::{Command, Stdio};
    /// use std::io::{self, Write};
    ///
    /// let output = Command::new("rev")
    ///     .stdin(Stdio::inherit())
    ///     .stdout(Stdio::piped())
    ///     .output()
    ///     .expect("Failed to execute command");
    ///
    /// print!("You piped in the reverse of: ");
    /// io::stdout().write_all(&output.stdout).unwrap();
    /// ```
    pub fn inherit() -> Stdio {
        Stdio {
            mode: StdioMode::Inherit,
        }
    }

    /// This stream will be ignored. This is the equivalent of attaching the
    /// stream to `/dev/null`.
    ///
    /// # Examples
    ///
    /// With stdout:
    ///
    /// ```no_run
    /// use std::process::{Command, Stdio};
    ///
    /// let output = Command::new("echo")
    ///     .arg("Hello, world!")
    ///     .stdout(Stdio::null())
    ///     .output()
    ///     .expect("Failed to execute command");
    ///
    /// assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    /// // Nothing echoed to console
    /// ```
    ///
    /// With stdin:
    ///
    /// ```no_run
    /// use std::process::{Command, Stdio};
    ///
    /// let output = Command::new("rev")
    ///     .stdin(Stdio::null())
    ///     .stdout(Stdio::piped())
    ///     .output()
    ///     .expect("Failed to execute command");
    ///
    /// assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    /// // Ignores any piped-in input
    /// ```
    pub fn null() -> Stdio {
        Stdio {
            mode: StdioMode::Null,
        }
    }
}
