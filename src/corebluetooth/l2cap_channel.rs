use std::{
    io::Result,
    mem::ManuallyDrop,
    os::fd::{FromRawFd, IntoRawFd, RawFd},
    pin::Pin,
    task::{Context, Poll},
};

use objc_foundation::INSData;
use objc_id::{Id, Shared};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::UnixStream,
};
use tracing::warn;

use super::types::{kCFStreamPropertySocketNativeHandle, CBL2CAPChannel, CFStream};
use crate::{error::ErrorKind, Error};

// This implementation is based upon the fact that that CBL2CAPChannel::outputStream -> an NS Output Stream; (https://developer.apple.com/documentation/foundation/outputstream)
// NS Output stream is toll free bridged to CFWriteStream (https://developer.apple.com/documentation/corefoundation/cfwritestream)
// CFWriteStream is a subclass of CFStream (https://developer.apple.com/documentation/corefoundation/cfstream?language=objc)
// CF Stream has properties (https://developer.apple.com/documentation/corefoundation/cfstream/stream_properties?language=objc)
// One of them is kCFStreamPropertySocketNativeHandle https://developer.apple.com/documentation/corefoundation/kcfstreampropertysocketnativehandle?language=objc
// kCFStreamPropertySocketNativeHandle is of type CFSocketNativeHandle https://developer.apple.com/documentation/corefoundation/cfsocketnativehandle?language=objc
// CFSocketNativeHandle is a property of CFSocket https://developer.apple.com/documentation/corefoundation/cfsocket?language=objc
// CF Socket is defined to be a bsd socket
// BSD Sockets are Unix Sockets on mac os

#[derive(Debug)]
pub struct Channel {
    _channel: Id<CBL2CAPChannel, Shared>,

    /// the _channel object owns the file descriptor, so we have a ManuallyDrop here to prevent the file descriptor from
    /// being double closed when this object is dropped.
    stream: ManuallyDrop<Pin<Box<UnixStream>>>,
}

enum ChannelCreationError {
    FileDescriptorPropertyNotValid,
    InputFileDescriptorBytesWrongSize,
    OutputFileDescriptorBytesWrongSize,
    FileDescriptorsNotIdentical,
    SetNonBlockingModeFailed(std::io::Error),
    TokioStreamCreation(std::io::Error),
}

impl Channel {
    pub fn new(channel: Id<CBL2CAPChannel, Shared>) -> crate::Result<Self> {
        let input_stream = channel.input_stream();
        let output_stream = channel.output_stream();

        let in_stream_prop = input_stream.property(&unsafe { kCFStreamPropertySocketNativeHandle });
        let out_stream_prop = output_stream.property(&unsafe { kCFStreamPropertySocketNativeHandle });

        let (Some(in_data), Some(out_data)) = (in_stream_prop, out_stream_prop) else {
            return Err(ChannelCreationError::FileDescriptorPropertyNotValid.into());
        };
        let in_bytes = in_data
            .bytes()
            .try_into()
            .map_err(|_| ChannelCreationError::InputFileDescriptorBytesWrongSize)?;
        let in_fd = RawFd::from_ne_bytes(in_bytes);

        let out_bytes = out_data
            .bytes()
            .try_into()
            .map_err(|_| ChannelCreationError::OutputFileDescriptorBytesWrongSize)?;
        let out_fd = RawFd::from_ne_bytes(out_bytes);

        if in_fd != out_fd {
            return Err(ChannelCreationError::FileDescriptorsNotIdentical.into());
        };

        let stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(in_fd) };
        stream
            .set_nonblocking(true)
            .map_err(ChannelCreationError::SetNonBlockingModeFailed)?;

        let tokio_stream = UnixStream::try_from(stream).map_err(ChannelCreationError::TokioStreamCreation)?;

        let stream = ManuallyDrop::new(Box::pin(tokio_stream));

        Ok(Channel {
            _channel: channel,
            stream,
        })
    }
}
impl AsyncRead for Channel {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.stream.as_mut().poll_read(cx, buf)
    }
}

impl AsyncWrite for Channel {
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

impl Drop for Channel {
    fn drop(&mut self) {
        // Manually deconstruct the tokio stream to a file descriptor to make sure it is not closed on drop.
        // The file descriptor is closed when _channel is dropped.

        // Safe as this is the drop impl, so the stream will not be used after this.
        let stream = unsafe { ManuallyDrop::take(&mut self.stream) };
        let stream = Pin::into_inner(stream);
        let Ok(stream) = stream.into_std() else {
            warn!("Could not convert tokio stream to standard stream");
            return;
        };
        let _stream = stream.into_raw_fd();
    }
}

impl From<ChannelCreationError> for Error {
    fn from(value: ChannelCreationError) -> Self {
        let message = match &value {
            ChannelCreationError::FileDescriptorPropertyNotValid => "File descriptor property not valid.",
            ChannelCreationError::InputFileDescriptorBytesWrongSize => {
                "Input file descriptor bytes are an invalid size."
            }
            ChannelCreationError::OutputFileDescriptorBytesWrongSize => {
                "Output file descriptor bytes are an invalid size."
            }
            ChannelCreationError::FileDescriptorsNotIdentical => "Input and output file descriptors are not identical.",
            ChannelCreationError::SetNonBlockingModeFailed(_) => "Could not get convert socket to async.",
            ChannelCreationError::TokioStreamCreation(_) => "Failed to create tokio unix socket.",
        };

        Error::new(
            ErrorKind::Internal,
            match value {
                ChannelCreationError::FileDescriptorPropertyNotValid
                | ChannelCreationError::InputFileDescriptorBytesWrongSize
                | ChannelCreationError::OutputFileDescriptorBytesWrongSize
                | ChannelCreationError::FileDescriptorsNotIdentical => None,
                ChannelCreationError::SetNonBlockingModeFailed(src)
                | ChannelCreationError::TokioStreamCreation(src) => Some(Box::new(src)),
            },
            message.to_owned(),
        )
    }
}
