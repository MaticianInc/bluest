use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

use bluedroid::l2cap_channel::{Channel as RawChannel, Reader as RawReader, Writer as RawWriter};

use crate::L2CapChannelImpl;

pub struct Channel {
    reader: Compat<RawReader>,
    writer: Compat<RawWriter>,
}

impl Channel {
    pub(crate) fn new(channel: RawChannel) -> Self {
        let (reader, writer) = channel.split();
        Self {
            reader: reader.compat(),
            writer: writer.compat_write(),
        }
    }
}

impl L2CapChannelImpl for Channel {
    fn split(self) -> (Reader, Writer) {
        let Self { reader, writer } = self;
        (reader, writer)
    }
}

impl AsyncRead for Channel {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        pin!(&mut self.reader).poll_read(cx, buf)
    }
}

impl AsyncWrite for Channel {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, std::io::Error>> {
        pin!(&mut self.writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        pin!(&mut self.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        pin!(&mut self.writer).poll_flush(cx)
    }
}

pub type Reader = Compat<RawReader>;
pub type Writer = Compat<RawWriter>;
