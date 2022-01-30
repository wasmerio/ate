#![allow(dead_code)]
use std::{result::Result};
use tokio::{io, io::AsyncReadExt, io::AsyncWriteExt};
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::os::unix::io::AsRawFd;
use libc::{
    tcsetattr, termios, ECHO, ICANON, ICRNL, IEXTEN, ISIG, IXON, OPOST, TCSANOW,
};

const MAX_MPSC: usize = std::usize::MAX >> 3;

pub struct Tty {
}

impl Tty {
    pub async fn stdin() -> Result<Stdin, std::io::Error> {
        let tty = tokio::fs::File::open("/dev/tty").await?;
        let fd = tty.as_raw_fd();

        let mut termios = std::mem::MaybeUninit::<termios>::uninit();
        io_result(unsafe { ::libc::tcgetattr(fd, termios.as_mut_ptr()) })?;
        let termios = unsafe { termios.assume_init() };
        let mut new_termios = termios.clone();

        new_termios.c_lflag &= !ECHO;
        new_termios.c_lflag &= !ICANON;
        new_termios.c_lflag &= !ISIG;
        new_termios.c_lflag &= !IXON;
        new_termios.c_lflag &= !IEXTEN;
        new_termios.c_lflag &= !ICRNL;
        new_termios.c_lflag &= !OPOST;

        unsafe { tcsetattr(fd, TCSANOW, &new_termios) };

        Ok(Stdin {
            tty,
            tty_fd: fd,
            termios,
        })
    }

    pub async fn stdout() -> Result<Stdout, io::Error> {
        let stdout = tokio::io::stdout();

        Ok(Stdout {
            stdout
        })
    }

    pub async fn stderr() -> Result<Stderr, io::Error> {
        let stderr = tokio::io::stdout();

        Ok(Stderr {
            stderr
        })
    }
}

pub struct Stdin
{
    tty: tokio::fs::File,
    tty_fd: i32,
    termios: termios,
}

impl Stdin
{
    pub async fn read(&mut self) -> Option<Vec<u8>> {
        let mut buffer = [0; 1024];
        if let Ok(read) = self.tty.read(&mut buffer[..]).await {
            let ret = (&buffer[0..read]).to_vec();
            Some(ret)
        } else {
            None
        }
    }

    pub async fn wait_for_flush(&mut self) -> Option<()> {
        None
    }
}

impl Drop
for Stdin
{
    fn drop(&mut self) {
        unsafe { tcsetattr(self.tty_fd, TCSANOW, &self.termios) };
    }
}

pub struct Stdout
{
    stdout: io::Stdout
}

impl Stdout
{
    pub async fn write(&mut self, data: Vec<u8>) -> Result<usize, io::Error> {
        self.stdout.write_all(&data[..]).await?;
        Ok(data.len())
    }

    pub async fn flush(&mut self) -> Result<(), io::Error> {
        self.stdout.flush().await?;
        Ok(())
    }
}

pub struct Stderr
{
    stderr: io::Stdout
}

impl Stderr
{
    pub async fn write(&mut self, data: Vec<u8>) -> Result<usize, io::Error> {
        self.stderr.write_all(&data[..]).await?;
        Ok(data.len())
    }

    pub async fn flush(&mut self) -> Result<(), io::Error> {
        self.stderr.flush().await?;
        Ok(())
    }
}

pub fn io_result(ret: libc::c_int) -> std::io::Result<()> {
    match ret {
        0 => Ok(()),
        _ => Err(std::io::Error::last_os_error()),
    }
}