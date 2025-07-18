use crate::error::ErrorKind;

pub mod adapter;
pub mod characteristic;
pub mod descriptor;
pub mod device;
pub mod service;

#[cfg(feature = "l2cap")]
pub mod l2cap_channel;

/// A platform-specific device identifier.
/// On android it contains the Bluetooth address in the format `AB:CD:EF:01:23:45`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceId(pub(crate) String);

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl From<bluedroid::GattError> for crate::Error {
    fn from(err: bluedroid::GattError) -> Self {
        use bluedroid::{error::BluetoothStatusCode, GattError};
        let message = err.to_string();
        crate::Error::new(
            match &err {
                GattError::GattReadNotPermitted
                | GattError::GattWriteNotPermitted
                | GattError::GattInsufficientAuthentication
                | GattError::GattInsufficientEncryption
                | GattError::GattInsufficientAuthorization
                | GattError::BluetoothStatusCode(BluetoothStatusCode::NotAllowed)
                | GattError::BluetoothStatusCode(BluetoothStatusCode::GattWriteNotAllowed) => ErrorKind::NotAuthorized,
                GattError::GattRequestNotSupported | GattError::BluetoothStatusCode(BluetoothStatusCode::NotBonded) => {
                    ErrorKind::NotSupported
                }
                GattError::GattInvalidOffset | GattError::GattInvalidAttributeLength => ErrorKind::InvalidParameter,
                GattError::GattConnectionCongested
                | GattError::GattConnectionTimeout
                | GattError::BluetoothStatusCode(BluetoothStatusCode::GattWriteBusy) => ErrorKind::Timeout,
                GattError::GattFailure
                | GattError::BluetoothStatusCode(BluetoothStatusCode::ProfileServiceNotBound)
                | GattError::BluetoothStatusCode(BluetoothStatusCode::FeatureNotSupported)
                | GattError::BluetoothStatusCode(BluetoothStatusCode::FeatureNotConfigured) => ErrorKind::Other,
                GattError::UnknownError(_)
                | GattError::BluetoothStatusCode(BluetoothStatusCode::Unknown)
                | GattError::BluetoothStatusCode(BluetoothStatusCode::UnknownError(_))
                | GattError::JavaError(_)
                | GattError::WritingToCCCDescriptor
                | GattError::NotExecuted => ErrorKind::Internal,
                GattError::BluetoothStatusCode(BluetoothStatusCode::NotEnabled)
                | GattError::BluetoothStatusCode(BluetoothStatusCode::MissingBluetoothConnectPermission) => {
                    ErrorKind::AdapterUnavailable
                }
                GattError::NotConnected => ErrorKind::NotConnected,
            },
            Some(Box::new(err)),
            message,
        )
    }
}

impl From<bluedroid::JavaError> for crate::Error {
    fn from(err: bluedroid::JavaError) -> Self {
        let message = format!("{err:?}");
        crate::Error::new(ErrorKind::Internal, None, message)
    }
}

impl From<bluedroid::scan::ScanError> for crate::Error {
    fn from(err: bluedroid::scan::ScanError) -> Self {
        use bluedroid::scan::ScanError;

        let message = err.to_string();
        crate::Error::new(
            match err {
                ScanError::AlreadyStarted => ErrorKind::AlreadyScanning,
                ScanError::FeatureUnsupported => ErrorKind::NotSupported,
                ScanError::ApplicationRegistration | ScanError::InteralError => ErrorKind::Internal,
                ScanError::OutOfHardwareResources => ErrorKind::NotReady,
                ScanError::ScanningToFrequently => ErrorKind::Timeout,
                ScanError::Unavailable => ErrorKind::AdapterUnavailable,
                ScanError::Unknown(_) => ErrorKind::Other,
            },
            Some(Box::new(err)),
            message,
        )
    }
}

impl From<bluedroid::device::PairingError> for crate::Error {
    fn from(err: bluedroid::device::PairingError) -> Self {
        use bluedroid::device::PairingError;

        let message = err.to_string();

        crate::Error::new(
            match err {
                PairingError::PairingError => ErrorKind::ConnectionFailed,
                PairingError::NotConnected => ErrorKind::NotConnected,
                PairingError::JavaError(_) | PairingError::Unknown(_) => ErrorKind::Other,
            },
            Some(Box::new(err)),
            message,
        )
    }
}
