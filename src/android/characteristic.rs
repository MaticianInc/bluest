use std::{hash::Hash, sync::Arc};

use async_lock::Mutex;
use futures_lite::{Stream, StreamExt};
use uuid::Uuid;

use bluedroid::characteristic::WriteType;

use crate::{error::ErrorKind, CharacteristicProperties, Descriptor, Error, Result};

use super::descriptor::DescriptorImpl;

#[derive(Debug, Clone)]
pub struct CharacteristicImpl {
    characteristic: bluedroid::Characteristic,
    cached_value: Arc<Mutex<Option<Vec<u8>>>>,
}

impl CharacteristicImpl {
    pub(crate) fn new(characteristic: bluedroid::Characteristic) -> Self {
        Self {
            characteristic,
            cached_value: Default::default(),
        }
    }
    pub fn uuid(&self) -> Uuid {
        self.characteristic.uuid()
    }

    pub async fn uuid_async(&self) -> Result<Uuid> {
        Ok(self.uuid())
    }

    pub async fn properties(&self) -> Result<CharacteristicProperties> {
        let properties = self.characteristic.properties();
        Ok(properties.into())
    }

    pub async fn value(&self) -> Result<Vec<u8>> {
        let cache = self.cached_value.lock().await;
        if let Some(val) = cache.as_ref() {
            return Ok(val.clone());
        }
        drop(cache);
        self.read().await
    }

    pub async fn read(&self) -> Result<Vec<u8>> {
        // This does block concurrent reads, but they are blocked by android anyway.
        let mut cache = self.cached_value.lock().await;
        let value = self.characteristic.read().await?.into_vec();
        *cache = Some(value.clone());
        Ok(value)
    }

    pub async fn write(&self, value: &[u8]) -> Result<()> {
        Ok(self.characteristic.write(WriteType::Default, value).await?)
    }

    pub async fn write_without_response(&self, value: &[u8]) -> Result<()> {
        Ok(self.characteristic.write(WriteType::NoResponse, value).await?)
    }

    /// Android does not give a way to get this
    pub fn max_write_len(&self) -> Result<usize> {
        Err(Error::from(ErrorKind::NotSupported))
    }

    pub async fn max_write_len_async(&self) -> Result<usize> {
        Err(Error::from(ErrorKind::NotSupported))
    }

    pub async fn notify(&self) -> Result<impl Stream<Item = Result<Vec<u8>>> + Send + Unpin + '_> {
        Ok(self.characteristic.notify().await?.map(|data| Ok(data.into_vec())))
    }

    pub async fn is_notifying(&self) -> Result<bool> {
        const CCC_DESCRIPTOR: Uuid = uuid::uuid!("00002902-0000-1000-8000-00805f9b34fb");
        let descriptor = self.characteristic.get_descriptor(CCC_DESCRIPTOR).unwrap();
        let ccc_descriptor_content = descriptor.read().await?;
        assert_eq!(ccc_descriptor_content.len(), 2);
        Ok((ccc_descriptor_content[0] & 0x01) != 0)
    }

    pub async fn discover_descriptors(&self) -> Result<Vec<Descriptor>> {
        Ok(self
            .characteristic
            .descriptors()
            .into_iter()
            .map(|descriptor| Descriptor(DescriptorImpl(descriptor)))
            .collect())
    }

    pub async fn descriptors(&self) -> Result<Vec<Descriptor>> {
        self.discover_descriptors().await
    }
}

impl From<bluedroid::characteristic::CharacteristicProperties> for CharacteristicProperties {
    fn from(properties: bluedroid::characteristic::CharacteristicProperties) -> Self {
        Self {
            broadcast: properties.broadcast(),
            read: properties.read(),
            write_without_response: properties.write_no_response(),
            write: properties.write(),
            notify: properties.notify(),
            indicate: properties.indicate(),
            authenticated_signed_writes: properties.signed_write(),
            extended_properties: properties.extended_props(),
            // Android does not seem to support these
            // https://developer.android.com/reference/android/bluetooth/BluetoothGattCharacteristic
            reliable_write: false,
            writable_auxiliaries: false,
        }
    }
}

impl PartialEq for CharacteristicImpl {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.characteristic, &other.characteristic)
    }
}

impl Eq for CharacteristicImpl {}

impl Hash for CharacteristicImpl {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&self.characteristic, state)
    }
}
