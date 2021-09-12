#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::pin::Pin;
use futures_util::stream::Stream;
use std::io;
use core::task::{Context, Poll};

use hyper;

use super::stream::*;

pub struct HyperAcceptor<'a>
{
    pub acceptor: Pin<Box<dyn Stream<Item = Result<HyperStream, io::Error>> + 'a>>,
}

impl hyper::server::accept::Accept for HyperAcceptor<'_> {
    type Conn = HyperStream;
    type Error = io::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        Pin::new(&mut self.acceptor).poll_next(cx)
    }
}