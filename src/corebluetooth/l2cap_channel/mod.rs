pub use futures_channel::FuturesCFStream as Channel;
mod futures_channel;

#[cfg(all(feature = "l2cap", feature = "tokio"))]
pub use tokio_channel::TokioL2CapChannel;
#[cfg(all(feature = "l2cap", feature = "tokio"))]
mod tokio_channel;
