use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::can::{CanNode, CanSystem};
use crate::websocket::DeviceStatus;

#[derive(Debug, Clone)]
pub struct TrackedDevice {
    pub node: CanNode,
    pub system: CanSystem,
    pub name: &'static str,
    pub online: bool,
    pub last_seen: Option<Instant>,
    pub last_error: Option<String>,
}

impl TrackedDevice {
    pub fn new(node: CanNode, system: CanSystem, name: &'static str) -> Self {
        Self {
            node,
            system,
            name,
            online: false,
            last_seen: None,
            last_error: None,
        }
    }

    pub fn status(&self, now: Instant) -> DeviceStatus {
        DeviceStatus {
            node_id: u8::from(self.node),
            name: self.name.to_string(),
            system: system_name(self.system).to_string(),
            online: self.online,
            last_seen_ms_ago: self
                .last_seen
                .map(|last_seen| now.duration_since(last_seen).as_millis() as u64),
            last_error: self.last_error.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceRegistry {
    devices: HashMap<CanNode, TrackedDevice>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        let mut devices = HashMap::new();

        for device in known_devices() {
            devices.insert(device.node, device);
        }

        Self { devices }
    }

    pub fn get(&self, node: CanNode) -> Option<&TrackedDevice> {
        self.devices.get(&node)
    }

    pub fn get_mut(&mut self, node: CanNode) -> Option<&mut TrackedDevice> {
        self.devices.get_mut(&node)
    }

    pub fn mark_seen(&mut self, node: CanNode, now: Instant) -> Option<&TrackedDevice> {
        let device = self.devices.get_mut(&node)?;
        device.online = true;
        device.last_seen = Some(now);
        device.last_error = None;
        Some(device)
    }

    pub fn mark_offline(&mut self, node: CanNode, error: Option<String>) -> Option<&TrackedDevice> {
        let device = self.devices.get_mut(&node)?;
        device.online = false;
        device.last_error = error;
        Some(device)
    }

    pub fn mark_timeouts(&mut self, now: Instant, timeout: Duration) {
        for device in self.devices.values_mut() {
            let is_stale = device
                .last_seen
                .map(|last_seen| now.duration_since(last_seen) > timeout)
                .unwrap_or(true);

            if is_stale {
                device.online = false;
            }
        }
    }

    pub fn online(&self) -> impl Iterator<Item = &TrackedDevice> {
        self.devices.values().filter(|device| device.online)
    }

    pub fn offline(&self) -> impl Iterator<Item = &TrackedDevice> {
        self.devices.values().filter(|device| !device.online)
    }

    pub fn snapshot(&self, now: Instant) -> Vec<DeviceStatus> {
        let mut devices = self
            .devices
            .values()
            .map(|device| device.status(now))
            .collect::<Vec<_>>();
        devices.sort_by_key(|device| device.node_id);
        devices
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn known_devices() -> [TrackedDevice; 9] {
    [
        TrackedDevice::new(CanNode::FrontLeft, CanSystem::Daq, "Front Left DAQ"),
        TrackedDevice::new(CanNode::FrontRight, CanSystem::Daq, "Front Right DAQ"),
        TrackedDevice::new(CanNode::RearLeft, CanSystem::Daq, "Rear Left DAQ"),
        TrackedDevice::new(CanNode::RearRight, CanSystem::Daq, "Rear Right DAQ"),
        TrackedDevice::new(CanNode::Nucleo1, CanSystem::Daq, "DAQ Nucleo 1"),
        TrackedDevice::new(CanNode::Nucleo2, CanSystem::Daq, "DAQ Nucleo 2"),
        TrackedDevice::new(CanNode::Dash, CanSystem::Daq, "Dash"),
        TrackedDevice::new(CanNode::Vcu, CanSystem::Vcu, "VCU"),
        TrackedDevice::new(CanNode::Bms, CanSystem::Bms, "BMS"),
    ]
}

fn system_name(system: CanSystem) -> &'static str {
    match system {
        CanSystem::Bms => "bms",
        CanSystem::Daq => "daq",
        CanSystem::Vcu => "vcu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_devices_initialize_offline() {
        let registry = DeviceRegistry::new();

        assert_eq!(registry.online().count(), 0);
        assert!(registry.offline().count() > 0);
        assert!(!registry.get(CanNode::Bms).expect("bms exists").online);
    }

    #[test]
    fn mark_seen_makes_device_online_and_updates_last_seen() {
        let mut registry = DeviceRegistry::new();
        let now = Instant::now();

        registry.mark_seen(CanNode::Bms, now).expect("bms exists");

        let bms = registry.get(CanNode::Bms).expect("bms exists");
        assert!(bms.online);
        assert_eq!(bms.last_seen, Some(now));
    }

    #[test]
    fn timeout_marks_stale_devices_offline() {
        let mut registry = DeviceRegistry::new();
        let now = Instant::now();

        registry.mark_seen(CanNode::Bms, now).expect("bms exists");
        registry.mark_timeouts(now + Duration::from_secs(3), Duration::from_secs(2));

        assert!(!registry.get(CanNode::Bms).expect("bms exists").online);
    }

    #[test]
    fn online_and_offline_iterators_track_status() {
        let mut registry = DeviceRegistry::new();
        let now = Instant::now();

        registry.mark_seen(CanNode::Bms, now).expect("bms exists");
        registry
            .mark_seen(CanNode::FrontLeft, now)
            .expect("daq exists");

        let online = registry
            .online()
            .map(|device| device.node)
            .collect::<Vec<_>>();

        assert!(online.contains(&CanNode::Bms));
        assert!(online.contains(&CanNode::FrontLeft));
        assert!(
            registry
                .offline()
                .all(|device| !online.contains(&device.node))
        );
    }
}
