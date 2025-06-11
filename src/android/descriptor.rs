use crate::{Result, Uuid};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DescriptorImpl(pub(super) bluedroid::Descriptor);

impl DescriptorImpl {
    pub fn uuid(&self) -> Uuid {
        self.0.uuid().unwrap()
    }

    pub async fn uuid_async(&self) -> Result<Uuid> {
        Ok(self.0.uuid()?)
    }

    pub async fn value(&self) -> Result<Vec<u8>> {
        self.read().await
    }

    pub async fn read(&self) -> Result<Vec<u8>> {
        Ok(self.0.read().await?.into_vec())
    }

    pub async fn write(&self, value: &[u8]) -> Result<()> {
        Ok(self.0.write(value).await?)
    }
}
