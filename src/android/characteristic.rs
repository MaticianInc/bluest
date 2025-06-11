use futures_lite::{Stream, StreamExt};
use uuid::Uuid;

use bluedroid::characteristic::WriteType;

use crate::{CharacteristicProperties, Descriptor, Result};

use super::descriptor::DescriptorImpl;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharacteristicImpl(pub(super) bluedroid::Characteristic);

impl CharacteristicImpl {
    pub fn uuid(&self) -> Uuid {
        self.0.uuid().unwrap()
    }

    pub async fn uuid_async(&self) -> Result<Uuid> {
        Ok(self.0.uuid()?)
    }

    pub async fn properties(&self) -> Result<CharacteristicProperties> {
        let properties = self.0.properties();
        Ok(properties.into())
    }

    pub async fn value(&self) -> Result<Vec<u8>> {
        self.read().await
    }

    pub async fn read(&self) -> Result<Vec<u8>> {
        Ok(self.0.read().await?.into_vec())
    }

    pub async fn write(&self, value: &[u8]) -> Result<()> {
        Ok(self.0.write(WriteType::Default, value).await?)
    }

    pub async fn write_without_response(&self, value: &[u8]) -> Result<()> {
        Ok(self.0.write(WriteType::NoResponse, value).await?)
    }

    /// Android does not give a way to get this
    pub fn max_write_len(&self) -> Result<usize> {
        Ok(usize::MAX)
    }

    pub async fn max_write_len_async(&self) -> Result<usize> {
        Ok(usize::MAX)
    }

    pub async fn notify(&self) -> Result<impl Stream<Item = Result<Vec<u8>>> + Send + Unpin + '_> {
        Ok(self.0.notify().await?.map(|data| Ok(data.into_vec())))
    }

    pub async fn is_notifying(&self) -> Result<bool> {
        const CCC_DESCRIPTOR: Uuid = uuid::uuid!("00002902-0000-1000-8000-00805f9b34fb");
        let descriptor = self.0.get_descriptor(CCC_DESCRIPTOR)?.unwrap();
        let ccc_descriptor_content = descriptor.read().await?;
        assert_eq!(ccc_descriptor_content.len(), 2);
        Ok((ccc_descriptor_content[0] & 0x01) != 0)
    }

    pub async fn discover_descriptors(&self) -> Result<Vec<Descriptor>> {
        Ok(self
            .0
            .descriptors()?
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
