use futures_core::Stream;
use futures_lite::StreamExt;
use uuid::Uuid;

#[cfg(feature = "l2cap")]
use super::service::ServiceImpl;
use crate::error::ErrorKind;
use crate::pairing::PairingAgent;
use crate::{DeviceId, Result, Service, ServicesChanged};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeviceImpl {
    pub(super) device: bluedroid::Device,
    // Android needs this to query the connection state
    pub(super) adapter: bluedroid::Adapter,
}

impl std::fmt::Display for DeviceImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //f.write_str(self.name().as_deref().unwrap_or("(Unknown)"))
        write!(f, "{:?}", self.device)
    }
}

impl DeviceImpl {
    pub fn id(&self) -> DeviceId {
        DeviceId(self.device.id().unwrap())
    }

    pub fn name(&self) -> Result<String> {
        self.device
            .name()?
            .ok_or_else(|| crate::Error::new(ErrorKind::NotFound, None, "Device Name Unavailable"))
    }

    pub async fn name_async(&self) -> Result<String> {
        self.name()
    }

    pub async fn is_connected(&self) -> bool {
        let Ok(connected_devices) = self.adapter.connected_devices() else {
            return false;
        };

        connected_devices
            .into_iter()
            .find(|device| device.id().is_ok_and(|id| id == self.device.id().unwrap()))
            .is_some()
    }

    pub async fn is_paired(&self) -> Result<bool> {
        Ok(self.device.paired()?)
    }

    pub async fn pair(&self) -> Result<()> {
        Ok(self.device.pair().await?)
    }

    pub async fn pair_with_agent<T: PairingAgent + 'static>(&self, _agent: &T) -> Result<()> {
        unimplemented!("Android does not support pairing Agents")
    }

    pub async fn unpair(&self) -> Result<()> {
        unimplemented!("Android does not support unpairing")
    }

    pub async fn discover_services(&self) -> Result<Vec<Service>> {
        Ok(self
            .device
            .discover_services()
            .await?
            .into_iter()
            .map(|service| Service(ServiceImpl(service)))
            .collect())
    }

    pub async fn discover_services_with_uuid(&self, uuid: Uuid) -> Result<Vec<Service>> {
        Ok(self
            .discover_services()
            .await?
            .into_iter()
            .filter(|service| service.0.uuid() == uuid)
            .collect())
    }

    pub async fn services(&self) -> Result<Vec<Service>> {
        self.discover_services().await
    }

    pub async fn service_changed_indications(
        &self,
    ) -> Result<impl Stream<Item = Result<ServicesChanged>> + Send + Unpin + '_> {
        Ok(self
            .device
            .services_changed()
            .map(|()| Ok(ServicesChanged(ServicesChangedImpl))))
    }

    pub async fn rssi(&self) -> Result<i16> {
        Ok(self.device.rssi().await?.try_into().unwrap())
    }

    #[cfg(feature = "l2cap")]
    pub async fn open_l2cap_channel(&self, psm: u16, secure: bool) -> Result<crate::L2CapChannel> {
        Ok(self.device.open_l2cap_channel(psm, secure)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServicesChangedImpl;

impl ServicesChangedImpl {
    pub fn was_invalidated(&self, _service: &Service) -> bool {
        true
    }
}
