#![cfg(feature = "l2cap")]

use futures_io::{AsyncRead, AsyncWrite};

use crate::{sys, Result};

/// A Bluetooth LE L2CAP Connection-oriented Channel (CoC)
pub type L2CapChannel = sys::l2cap_channel::Channel;

pub trait L2CapChannelImpl: AsyncRead + AsyncWrite {
    fn split(self) -> (L2CapReader, L2CapWriter);
}

pub type L2CapReader = sys::l2cap_channel::Reader;

trait L2CapReaderImpl: AsyncRead {}

impl L2CapReaderImpl for L2CapReader {}

pub type L2CapWriter = sys::l2cap_channel::Writer;

trait L2CapWriterImpl: AsyncWrite {}

impl L2CapWriterImpl for L2CapWriter {}
