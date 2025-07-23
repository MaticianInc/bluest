use std::io::Result;
use std::pin::{pin, Pin};
use std::task::{Context, Poll};

use bluer::l2cap::{SocketAddr, Stream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tracing::trace;

use crate::error::ErrorKind;
use crate::L2CapChannelImpl;

pub use bluer::l2cap::stream::{OwnedReadHalf as Reader, OwnedWriteHalf as Writer};

const SECURE_CHANNEL_KEY_SIZE: u8 = 16;

#[derive(Debug)]
pub struct Channel {
    stream: bluer::l2cap::Stream,
}

enum ChannelCreationError {
    SetSecurityError(std::io::Error),
    ConnectionError(std::io::Error),
}

impl Channel {
    pub async fn new(sa: SocketAddr, secure: bool) -> crate::Result<Self> {
        let stream = Stream::connect(sa)
            .await
            .map_err(ChannelCreationError::ConnectionError)?;

        if secure {
            stream
                .as_ref()
                .set_security(bluer::l2cap::Security {
                    level: bluer::l2cap::SecurityLevel::High,
                    key_size: SECURE_CHANNEL_KEY_SIZE,
                })
                .map_err(ChannelCreationError::SetSecurityError)?;
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

        Ok(Self { stream })
    }
}

impl L2CapChannelImpl for Channel {
    fn split(self) -> (crate::L2CapReader, crate::L2CapWriter) {
        self.stream.into_split()
    }
}

impl AsyncRead for Channel {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        pin!(&mut self.stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for Channel {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        pin!(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        pin!(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        pin!(&mut self.stream).poll_shutdown(cx)
    }
}

impl From<ChannelCreationError> for crate::Error {
    fn from(value: ChannelCreationError) -> Self {
        let kind = match &value {
            ChannelCreationError::SetSecurityError(_) => ErrorKind::Internal,
            ChannelCreationError::ConnectionError(_) => ErrorKind::ConnectionFailed,
        };
        let message = match &value {
            ChannelCreationError::SetSecurityError(_) => "Error setting connection security level.",
            ChannelCreationError::ConnectionError(_) => "Error connecting to l2cap stream.",
        };
        crate::Error::new(
            kind,
            match value {
                ChannelCreationError::SetSecurityError(io) | ChannelCreationError::ConnectionError(io) => {
                    Some(Box::new(io))
                }
            },
            message.to_owned(),
        )
    }
}
