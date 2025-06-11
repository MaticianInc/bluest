use crate::error::ErrorKind;

pub mod adapter;
pub mod characteristic;
pub mod descriptor;
pub mod device;
pub mod l2cap_channel;
pub mod service;

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
        use bluedroid::GattError;
        let message = err.to_string();
        crate::Error::new(
            match &err {
                GattError::GattReadNotPermitted
                | GattError::GattWriteNotPermitted
                | GattError::GattInsufficientAuthentication
                | GattError::GattInsufficientEncryption
                | GattError::GattInsufficientAuthorization => ErrorKind::NotAuthorized,
                GattError::GattRequestNotSupported => ErrorKind::NotSupported,
                GattError::GattInvalidOffset | GattError::GattInvalidAttributeLength => ErrorKind::InvalidParameter,
                GattError::GattConnectionCongested | GattError::GattConnectionTimeout => ErrorKind::Timeout,
                GattError::GattFailure => ErrorKind::Other,
                GattError::UnknownError(_) | GattError::JavaError(_) | GattError::WritingToCCCDescriptor => {
                    ErrorKind::Internal
                }
            },
            Some(Box::new(err)),
            message,
        )
    }
}

impl From<bluedroid::JavaError> for crate::Error {
    fn from(err: bluedroid::JavaError) -> Self {
        let message = format!("{:?}", err);
        crate::Error::new(ErrorKind::Internal, None, message)
    }
}
