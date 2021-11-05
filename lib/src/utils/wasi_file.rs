use std::fs::File;
use std::io::{Read, Write};
use wasi::*;
use tokio::io::{ReadBuf, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::time::Duration;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
#[cfg(unix)]
use std::os::unix::io::{FromRawFd, AsRawFd, RawFd};
#[cfg(target_os = "wasi")]
use std::os::wasi::io::{FromRawFd, AsRawFd, RawFd};

pub struct WasiFile
{
    file: File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PollType
{
    Read,
    Write
}

impl WasiFile
{
    pub fn new(file: File) -> WasiFile
    {
        WasiFile {
            file,
        }
    }

    pub fn poll(&self, type_: PollType) -> bool
    {
        unsafe {
            let fd = self.file.as_raw_fd();
            let subscription = Subscription {
                userdata: 0,
                u: SubscriptionU {
                    tag: match type_ {
                        PollType::Read => EVENTTYPE_FD_READ,
                        PollType::Write => EVENTTYPE_FD_WRITE
                    },
                    u: match type_ {
                        PollType::Read => SubscriptionUU {
                            fd_read: SubscriptionFdReadwrite { file_descriptor: fd as Fd },
                        },
                        PollType::Write => SubscriptionUU {
                            fd_write: SubscriptionFdReadwrite { file_descriptor: fd as Fd }
                        }
                    }
                }
            };
            
            let mut evt = Event {
                userdata: 0,
                error: 0,
                r#type: 0,
                fd_readwrite: EventFdReadwrite {
                    nbytes: 0,
                    flags: 0
                }
            };
            
            let raw_sub = &subscription as *const Subscription;
            let raw_evt = &mut evt as &mut Event;
            if let Ok(nevts) = poll_oneoff(raw_sub, raw_evt, 1) {
                if evt.fd_readwrite.flags == EVENTRWFLAGS_FD_READWRITE_HANGUP {
                    return true;
                }
                match evt.r#type {
                    EVENTTYPE_FD_READ if type_ == PollType::Read => true,
                    EVENTTYPE_FD_WRITE if type_ == PollType::Write => true,
                    _ => false
                }
            } else {
                false
            }
        }
    }

    fn handle_waker(&mut self, waker: Waker)
    {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            waker.wake();
        });
    }
}

impl AsyncRead
for WasiFile
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>>
    {
        if self.poll(PollType::Read) == false {
            self.handle_waker(cx.waker().clone());
            return Poll::Pending;
        }

        unsafe {
            let read = self.file.read(buf.initialize_unfilled());
            let read = read
                .map(|read| {
                    buf.advance(read);
                });
            Poll::Ready(read)
        }
    }
}

impl AsyncWrite
for WasiFile
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, tokio::io::Error>>
    {
        if self.poll(PollType::Write) == false {
            self.handle_waker(cx.waker().clone());
            return Poll::Pending;
        }

        unsafe {
            Poll::Ready(
                self.file.write(buf)
            )
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), tokio::io::Error>>
    {
        Poll::Ready(self.file.flush())
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), tokio::io::Error>>
    {
        Poll::Ready(Ok(()))
    }
}