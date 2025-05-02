#![cfg(feature = "l2cap")]

use bluer::l2cap::{SocketAddr, Stream};
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    io::Result,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead as TokioAsyncRead, AsyncWrite as TokioAsyncWrite, ReadBuf};
use tracing::trace;

use crate::error::{Error, ErrorKind};

const SECURE_CHANNEL_KEY_SIZE: u8 = 16;

#[derive(Debug)]
pub struct Channel {
    stream: Pin<Box<bluer::l2cap::Stream>>,
}

#[cfg(feature = "tokio")]
pub type TokioL2CapChannel = Channel;

impl Channel {
    pub async fn new(sa: SocketAddr, secure: bool) -> crate::Result<Self> {
        let stream = Stream::connect(sa).await.map_err(|e| {
            Error::new(
                ErrorKind::ConnectionFailed,
                Some(Box::new(e)),
                "Could not connect to l2cap stream.",
            )
        })?;

        if secure {
            stream
                .as_ref()
                .set_security(bluer::l2cap::Security {
                    level: bluer::l2cap::SecurityLevel::High,
                    key_size: SECURE_CHANNEL_KEY_SIZE,
                })
                .map_err(|e| {
                    Error::new(
                        ErrorKind::Internal,
                        Some(Box::new(e)),
                        "Could not set secutiry of l2cap stream",
                    )
                })?;
        }

        trace!(name: "Bluetooth Stream",
            "Local address: {:?}\n Remote address: {:?}\n Send MTU: {:?}\n Recv MTU: {:?}\n Security: {:?}\n Flow control: {:?}",
            stream.as_ref().local_addr(),
            stream.peer_addr(),
            stream.as_ref().send_mtu(),
            stream.as_ref().recv_mtu(),
            stream.as_ref().security(),
            stream.as_ref().flow_control(),
        );

        Ok(Self {
            stream: Box::pin(stream),
        })
    }
}

impl AsyncRead for Channel {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        let mut buf = ReadBuf::new(buf);
        match self.stream.as_mut().poll_read(cx, &mut buf)? {
            Poll::Ready(()) => Poll::Ready(Ok(buf.filled().len())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for Channel {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        TokioAsyncWrite::poll_write(self, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        TokioAsyncWrite::poll_flush(self, cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        TokioAsyncWrite::poll_shutdown(self, cx)
    }
}

impl TokioAsyncRead for Channel {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.stream.as_mut().poll_read(cx, buf)
    }
}

impl TokioAsyncWrite for Channel {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.stream.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.stream.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.stream.as_mut().poll_shutdown(cx)
    }
}
