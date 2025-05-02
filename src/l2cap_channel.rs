#![cfg(feature = "l2cap")]

use std::pin::Pin;

use crate::sys;

/// A Bluetooth LE L2CAP Connection-oriented Channel (CoC)
#[derive(Debug)]
pub struct L2capChannel {
    pub(crate) inner: Pin<Box<sys::l2cap_channel::Channel>>,
}

impl futures_io::AsyncRead for L2capChannel {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        tracing::trace!("Read Polling L2cap");
        self.inner.as_mut().poll_read(cx, buf)
    }
}

impl futures_io::AsyncWrite for L2capChannel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.inner.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        self.inner.as_mut().poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        self.inner.as_mut().poll_close(cx)
    }
}

/// A Bluetooth LE L2CAP Connection-oriented Channel (CoC)
/// using the `tokio` AsyncRead + AsyncWrite
#[cfg(feature = "tokio")]
#[derive(Debug)]
pub struct TokioL2CapChannel {
    pub(crate) inner: Pin<Box<sys::l2cap_channel::TokioL2CapChannel>>,
}

#[cfg(feature = "tokio")]
impl tokio::io::AsyncRead for TokioL2CapChannel {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        tracing::trace!("Read Polling L2cap");
        self.inner.as_mut().poll_read(cx, buf)
    }
}

#[cfg(feature = "tokio")]
impl tokio::io::AsyncWrite for TokioL2CapChannel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        self.inner.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.inner.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.inner.as_mut().poll_shutdown(cx)
    }
}
