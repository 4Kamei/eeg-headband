use bluest::{Adapter, Device};
use std::{str::FromStr, time::Duration};
use tokio::{task, time::timeout};
use uuid::Uuid;

/// Manages an adapter, scans for devices and automatically connects to peripherals that matches
pub struct BleDriver {
    discovery_handle: task::JoinHandle<()>,
}

impl BleDriver {
    pub fn new(adapter: Adapter) -> Self {
        let handle = tokio::spawn(discovery_task(adapter));

        Self {
            discovery_handle: handle,
        }
    }
}

async fn discovery_task(adapter: Adapter) -> () {
    //let service_uuid = Uuid::from_bytes(common::EEG_DATA_SERVICE_UUID);
    let service_uuid = Uuid::from_str("00001105-0000-1000-8000-00805f9b34fb").unwrap();
    let service_uuids = &[service_uuid.clone()];
    let stream = adapter.scan(service_uuids).await.unwrap();

    loop {
        for device in adapter.connected_devices().await.unwrap() {
            let name = device.name().unwrap_or_else(|_| "<Unknown>".to_string());

            tracing::info!(?name, "Found device");

            if !device.is_paired().await.unwrap() {
                tracing::info!("Waiting for device to be paired");
                continue;
            }

            let output = timeout(
                Duration::from_secs(100),
                device.discover_services_with_uuid(service_uuid.clone()),
            )
            .await;
            tracing::info!(?output, "Discovering services");
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}
