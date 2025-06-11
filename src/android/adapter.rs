use std::collections::HashMap;

use bluedroid::scan::scan_filter::{ScanFilter, ServiceUuid};
use futures_lite::{stream, Stream, StreamExt};
use tracing::error;
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
            adapter: self.0.clone(),
        }))
    }

    /// Finds all connected Bluetooth LE devices
    pub async fn connected_devices(&self) -> Result<Vec<Device>> {
        let adapter = self.0.clone();
        Ok(self
            .0
            .connected_devices()?
            .into_iter()
            .map(|device| {
                Device(DeviceImpl {
                    device,
                    adapter: adapter.clone(),
                })
            })
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
            let devices_services = device.discover_services().await?;
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
    pub async fn scan<'a>(
        &'a self,
        services: &'a [Uuid],
    ) -> Result<impl Stream<Item = AdvertisingDevice> + Send + Unpin + 'a> {
        let adapter = self.0.clone();
        let scan = self.0.scan(get_filters(services))?;
        Ok(scan.map(move |service| {
            AdvertisingDevice {
                device: Device(DeviceImpl {
                    device: service.device(),
                    adapter: adapter.clone(),
                }),
                adv_data: AdvertisementData {
                    local_name: match service.local_name() {
                        Ok(local_name) => Some(local_name),
                        Err(e) => {
                            error!("Could not get device name {:?}", e);
                            None
                        }
                    },
                    manufacturer_data: {
                        //Android seems to support having multiple manufacturer data elements. Panic if there is more then one.
                        // https://developer.android.com/reference/android/bluetooth/le/ScanRecord#getManufacturerSpecificData()
                        let mut manufacturer_datas = service.manufacturer_specific_data().unwrap().into_iter();
                        let manufacturer_data = manufacturer_datas.next().map(|(company_id, data)| ManufacturerData {
                            company_id: company_id.try_into().expect("Invalid company id"),
                            data: data.into_vec(),
                        });
                        assert!(
                            manufacturer_datas.next().is_none(),
                            "There should at most one manufacturer data entry"
                        );
                        manufacturer_data
                    },
                    services: service.get_service_uuids().unwrap_or_else(|e| {
                        error!("Could not get services {:?}", e);
                        Vec::new()
                    }),
                    service_data: match service.get_service_datas() {
                        Ok(service_data) => {
                            HashMap::from_iter(service_data.into_iter().map(|(key, data)| (key, data.into_vec())))
                        }
                        Err(e) => {
                            error!("Could not get service data {:?}", e);
                            HashMap::new()
                        }
                    },
                    tx_power_level: match service.tx_power_level() {
                        Ok(tx_power_level) => tx_power_level.map(|power_level| i16::try_from(power_level).unwrap()),
                        Err(e) => {
                            error!("Could not get tx_power_level{:?}", e);
                            None
                        }
                    },
                    is_connectable: service.is_connectable().unwrap_or_else(|e| {
                        error!("Could not get is_device_connectable{:?}", e);
                        false
                    }),
                },
                rssi: match service.rssi() {
                    Ok(rssi) => Some(i16::from(rssi)),
                    Err(e) => {
                        error!("Could not get device rssi {:?}", e);
                        None
                    }
                },
            }
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
        let adapter = self.0.clone();
        Ok(stream::iter(connected_devices)
            .chain(scan.map(move |scan_result| {
                Device(DeviceImpl {
                    device: scan_result.device(),
                    adapter: adapter.clone(),
                })
            }))
            .map(|device| Ok(device)))
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
        use bluedroid::device::ConnectionState;

        Ok(device
            .0
            .device
            .connection_events()
            .filter_map(|connection_state| match connection_state {
                ConnectionState::Connected => Some(ConnectionEvent::Connected),
                ConnectionState::Disconnected => Some(ConnectionEvent::Disconnected),
                ConnectionState::Disconnecting | ConnectionState::Connecting => None,
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
