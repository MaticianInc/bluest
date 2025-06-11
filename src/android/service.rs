use bluedroid::service::ServiceType;

use crate::{Characteristic, Result, Service, Uuid};

use super::characteristic::CharacteristicImpl;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceImpl(pub(super) bluedroid::Service);

impl ServiceImpl {
    pub fn uuid(&self) -> Uuid {
        self.0.uuid()
    }

    pub async fn uuid_async(&self) -> Result<Uuid> {
        Ok(self.uuid())
    }

    pub async fn is_primary(&self) -> Result<bool> {
        Ok(matches!(self.0.service_type(), ServiceType::Primary))
    }

    pub async fn discover_characteristics(&self) -> Result<Vec<Characteristic>> {
        Ok(self
            .0
            .characteristics()
            .into_iter()
            .map(|characteristic| Characteristic(CharacteristicImpl(characteristic)))
            .collect())
    }

    pub async fn discover_characteristics_with_uuid(&self, uuid: Uuid) -> Result<Vec<Characteristic>> {
        Ok(self
            .0
            .characteristics()
            .into_iter()
            .filter_map(|characteristic| {
                (characteristic.uuid() == uuid).then_some(Characteristic(CharacteristicImpl(characteristic)))
            })
            .collect())
    }

    pub async fn characteristics(&self) -> Result<Vec<Characteristic>> {
        self.discover_characteristics().await
    }

    pub async fn discover_included_services(&self) -> Result<Vec<Service>> {
        Ok(self
            .0
            .included_services()
            .into_iter()
            .map(|service| Service(Self(service)))
            .collect())
    }

    pub async fn discover_included_services_with_uuid(&self, uuid: Uuid) -> Result<Vec<Service>> {
        Ok(self
            .0
            .included_services()
            .into_iter()
            .filter_map(|service| (service.uuid() == uuid).then_some(Service(Self(service))))
            .collect())
    }

    pub async fn included_services(&self) -> Result<Vec<Service>> {
        self.discover_included_services().await
    }
}
