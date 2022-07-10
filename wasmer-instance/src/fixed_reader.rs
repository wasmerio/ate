use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use bytes::Bytes;
use tokio::io::AsyncRead;
use tokio::io::ReadBuf;
use ate_comms::StreamReadable;
use async_trait::async_trait;

pub struct FixedReader
{
    data: Option<Bytes>,
}

impl FixedReader
{
    pub fn new(data: Vec<u8>) -> FixedReader
    {
        FixedReader {
            data: Some(Bytes::from(data))
        }
    }
}

impl AsyncRead
for FixedReader
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>>
    {
        if let Some(data) = self.data.take() {
            if data.len() <= buf.remaining() {
                buf.put_slice(&data[..]);
            } else {
                let end = buf.remaining();
                buf.put_slice(&data[..end]);
                self.data.replace(data.slice(end..));
            }
        }
        Poll::Ready(Ok(()))
    }
}

#[async_trait]
impl StreamReadable
for FixedReader
{
    async fn read(&mut self) -> io::Result<Vec<u8>> {
        if let Some(data) = self.data.take() {
            Ok(data.to_vec())
        } else {
            Ok(Vec::new())
        }
    }
}
