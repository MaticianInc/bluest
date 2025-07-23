use std::collections::HashMap;

use bluedroid::{
    scan::{
        scan_filter::{ScanFilter, ServiceUuid},
        scan_result::ScanResult,
    },
    ConnectionState,
};
use futures_lite::{stream, Stream, StreamExt};
use tracing::{error, trace};
use uuid::Uuid;

use super::{device::DeviceImpl, DeviceId};
use crate::{AdapterEvent, AdvertisementData, AdvertisingDevice, ConnectionEvent, Device, ManufacturerData, Result};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AdapterImpl(bluedroid::Adapter);

impl AdapterImpl {
    /// Creates an interface to the default Bluetooth adapter for the system
    pub async fn default() -> Option<Self> {
        Some(Self(bluedroid::Adapter::default()))
    }

    /// A stream of [`AdapterEvent`] which allows the application to identify when the adapter is enabled or disabled.
    pub(crate) async fn events(&self) -> Result<impl Stream<Item = Result<AdapterEvent>> + Send + Unpin + '_> {
        Ok(stream::empty()) // TODO
    }

    /// Asynchronously blocks until the adapter is available
    pub async fn wait_available(&self) -> Result<()> {
        Ok(())
    }

    /// Attempts to create the device identified by `id`
    pub async fn open_device(&self, id: &DeviceId) -> Result<Device> {
        Ok(Device(DeviceImpl {
            device: self.0.open_device(id.0.as_str())?,
        }))
    }

    /// Finds all connected Bluetooth LE devices
    pub async fn connected_devices(&self) -> Result<Vec<Device>> {
        Ok(self
            .0
            .connected_devices()?
            .into_iter()
            .map(|device| Device(DeviceImpl { device }))
            .collect())
    }

    /// Finds all connected devices providing any service in `services`
    ///
    /// # Panics
    ///
    /// Panics if `services` is empty.
    pub async fn connected_devices_with_services(&self, services: &[Uuid]) -> Result<Vec<Device>> {
        assert!(!services.is_empty());

        let devices = self.connected_devices().await?;
        let mut devices_with_services = Vec::new();
        for device in devices.into_iter() {
            let devices_services = device.0.cached_services()?;
            if devices_services
                .iter()
                .any(|service| services.contains(&service.uuid()))
            {
                devices_with_services.push(device);
            }
        }
        Ok(devices_with_services)
    }

    /// Starts scanning for Bluetooth advertising packets.
    ///
    /// Returns a stream of [`AdvertisingDevice`] structs which contain the data from the advertising packet and the
    /// [`Device`] which sent it. Scanning is automatically stopped when the stream is dropped. Inclusion of duplicate
    /// packets is a platform-specific implementation detail.
    ///
    /// If `services` is not empty, returns advertisements including at least one GATT service with a UUID in
    /// `services`. Otherwise returns all advertisements.
    ///
    /// Android seems to support having multiple manufacturer data elements. Panic if there is more then one.
    /// https://developer.android.com/reference/android/bluetooth/le/ScanRecord#getManufacturerSpecificData()
    pub async fn scan<'a>(
        &'a self,
        services: &'a [Uuid],
    ) -> Result<impl Stream<Item = AdvertisingDevice> + Send + Unpin + 'a> {
        let scan = self.0.scan(get_filters(services))?;
        Ok(scan
            .take_while(|scan_result| {
                if let Err(e) = scan_result {
                    error!("Error while scanning {:?}", e);
                }
                scan_result.is_ok()
            })
            .filter_map(move |service| {
                let service = service.expect("We should be stopped by the takw ehile");
                if !check_advertising(&service) {
                    return None;
                }
                Some(AdvertisingDevice {
                    device: Device(DeviceImpl {
                        device: service.device(),
                    }),
                    adv_data: AdvertisementData {
                        local_name: service.local_name(),
                        manufacturer_data: {
                            let mut manufacturer_datas = service.manufacturer_specific_data().into_iter();
                            let manufacturer_data =
                                manufacturer_datas.next().map(|(company_id, data)| ManufacturerData {
                                    company_id: company_id.try_into().expect("Invalid company id"),
                                    data: data.into_vec(),
                                });
                            assert!(
                                manufacturer_datas.next().is_none(),
                                "There should at most one manufacturer data entry"
                            );
                            manufacturer_data
                        },
                        services: service.get_service_uuids(),
                        service_data: HashMap::from_iter(
                            service
                                .get_service_datas()
                                .into_iter()
                                .map(|(key, data)| (key, data.into_vec())),
                        ),
                        tx_power_level: service.tx_power_level().map(i16::from),
                        is_connectable: service.is_connectable(),
                    },
                    rssi: Some(i16::from(service.rssi())),
                })
            }))
    }

    /// Finds Bluetooth devices providing any service in `services`.
    ///
    /// Returns a stream of [`Device`] structs with matching connected devices returned first. If the stream is not
    /// dropped before all matching connected devices are consumed then scanning will begin for devices advertising any
    /// of the `services`. Scanning will continue until the stream is dropped. Inclusion of duplicate devices is a
    /// platform-specific implementation detail.
    pub async fn discover_devices<'a>(
        &'a self,
        services: &'a [Uuid],
    ) -> Result<impl Stream<Item = Result<Device>> + Send + Unpin + 'a> {
        let connected_devices = self.connected_devices_with_services(services).await?;
        let scan = self.0.scan(get_filters(services))?;
        Ok(
            stream::iter(connected_devices.into_iter().map(Ok)).chain(scan.filter_map(move |scan_result| {
                let scan_result = match scan_result {
                    Ok(scan_result) => scan_result,
                    Err(e) => {
                        return Some(Err(e.into()));
                    }
                };
                if !check_advertising(&scan_result) {
                    return None;
                }
                Some(Ok(Device(DeviceImpl {
                    device: scan_result.device(),
                })))
            })),
        )
    }

    /// Connects to the [`Device`]
    pub async fn connect_device(&self, device: &Device) -> Result<()> {
        Ok(device.0.device.connect().await?)
    }

    /// Disconnects from the [`Device`]
    pub async fn disconnect_device(&self, device: &Device) -> Result<()> {
        Ok(device.0.device.disconnect().await?)
    }

    /// Monitors a device for connection/disconnection events.
    #[inline]
    pub async fn device_connection_events<'a>(
        &'a self,
        device: &'a Device,
    ) -> Result<impl Stream<Item = ConnectionEvent> + Send + Unpin + 'a> {
        Ok(device
            .0
            .device
            .connection_events()
            .take_while(|cs| cs.is_ok())
            .filter_map(|connection_state| {
                let connection_state = connection_state.expect("Errors excluded by take_while");
                match connection_state {
                    ConnectionState::Connected => Some(ConnectionEvent::Connected),
                    ConnectionState::Disconnected => Some(ConnectionEvent::Disconnected),
                    ConnectionState::Disconnecting | ConnectionState::Connecting => None,
                }
            }))
    }
}

fn get_filters(services: &[Uuid]) -> Vec<ScanFilter> {
    services
        .iter()
        .map(|service| ScanFilter {
            service_uuid: Some(ServiceUuid {
                uuid: *service,
                mask: None,
            }),
            ..Default::default()
        })
        .collect()
}

// Android scans will show non-discoverable devices
fn check_advertising(device_result: &ScanResult) -> bool {
    let advertising_flags = match device_result.advertising_flags() {
        Some(advertising_flags) => advertising_flags,
        None => {
            return false;
        }
    };

    trace!(
        "Device {:?} advertising flags {:?}",
        device_result.local_name(),
        device_result.advertising_flags()
    );
    advertising_flags.le_discoverable() || advertising_flags.general_discoverable()
}
