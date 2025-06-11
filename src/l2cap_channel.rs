#![cfg(feature = "l2cap")]

use futures_io::{AsyncRead, AsyncWrite};

use crate::sys;

/// A Bluetooth LE L2CAP Connection-oriented Channel (CoC)
pub type L2CapChannel = sys::l2cap_channel::Channel;

/// Trait for functions that all L2Cap Channels have
pub trait L2CapChannelImpl: AsyncRead + AsyncWrite {
    /// Split the channel into a reader and write half
    fn split(self) -> (L2CapReader, L2CapWriter);
}

/// Reader half of Bluetooth LE L2CAP Connection-oriented Channel (CoC)
pub type L2CapReader = sys::l2cap_channel::Reader;

trait _L2CapReaderImpl: AsyncRead {}

impl _L2CapReaderImpl for L2CapReader {}

/// Writer half of Bluetooth LE L2CAP Connection-oriented Channel (CoC)
pub type L2CapWriter = sys::l2cap_channel::Writer;

trait _L2CapWriterImpl: AsyncWrite {}

impl _L2CapWriterImpl for L2CapWriter {}
