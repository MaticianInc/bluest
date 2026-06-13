use crate::Uuid;

pub mod adapter;
pub mod characteristic;
pub mod descriptor;
pub mod device;
pub mod error;
pub mod service;

#[cfg(feature = "l2cap")]
pub mod l2cap_channel;

mod delegates;
mod types;

/// A platform-specific device identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceId(Uuid);

impl DeviceId {
    /// Constructs a [`DeviceId`] from a CoreBluetooth peripheral identifier.
    ///
    /// Lets callers name a peripheral by its `CBPeripheral.identifier` (e.g. the
    /// `bluetoothIdentifier` AccessorySetupKit hands back) without first
    /// discovering it through a scan.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
