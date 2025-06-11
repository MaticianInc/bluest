#![cfg(feature = "l2cap")]

pub use bluedroid::l2cap_channel::{Channel, Reader, Writer};

use crate::l2cap_channel::{L2CapChannelImpl, L2CapReader, L2CapWriter};

impl L2CapChannelImpl for Channel {
    fn split(self) -> (L2CapReader, L2CapWriter) {
        self.split()
    }
}
