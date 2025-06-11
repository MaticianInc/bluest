use futures_core::Stream;
use futures_lite::StreamExt;
use uuid::Uuid;

use bluedroid::ConnectionState;

use crate::error::ErrorKind;
use crate::pairing::PairingAgent;
use crate::{DeviceId, Result, Service, ServicesChanged};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeviceImpl {
    pub(super) device: bluedroid::Device,
}

impl std::fmt::Display for DeviceImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //f.write_str(self.name().as_deref().unwrap_or("(Unknown)"))
        write!(f, "{:?}", self.device)
    }
}

impl DeviceImpl {
    pub fn id(&self) -> DeviceId {
        DeviceId(self.device.id())
    }

    pub fn name(&self) -> Result<String> {
        self.device
            .name()
            .ok_or_else(|| crate::Error::new(ErrorKind::NotFound, None, "Device Name Unavailable".to_owned()))
    }

    pub async fn name_async(&self) -> Result<String> {
        self.name()
    }

    pub async fn is_connected(&self) -> bool {
        // Use the client connection state as you need that for the device to be usable
        // Devices can be connected globally, but you still need to call connect for them
        // to be usable.
        matches!(self.device.client_connection_state(), ConnectionState::Connected)
    }

    pub async fn is_paired(&self) -> Result<bool> {
        Ok(self.device.paired())
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
            .map(|service| Service(super::service::ServiceImpl(service)))
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
        let cached_services = self.cached_services()?;
        match cached_services.is_empty() {
            true => self.discover_services().await,
            false => Ok(cached_services),
        }
    }

    pub(super) fn cached_services(&self) -> Result<Vec<Service>> {
        Ok(self
            .device
            .cached_services()?
            .into_iter()
            .map(|service| Service(super::service::ServiceImpl(service)))
            .collect())
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
