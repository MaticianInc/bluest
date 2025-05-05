pub use objc_foundation::stream::AsyncCFStream as Channel;

#[cfg(all(feature = "l2cap", feature = "tokio"))]
pub use tokio_channel::TokioL2CapChannel;
#[cfg(all(feature = "l2cap", feature = "tokio"))]
mod tokio_channel;
