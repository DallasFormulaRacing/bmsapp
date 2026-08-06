use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WsEnvelope<T> {
    pub system: System,
    pub device: Device,
    pub data: T,
}

pub type BackendEvent = WsEnvelope<BackendEventData>;

/// GUI -> can-bridge commands (same envelope shape as backend events).
pub type FrontendCommand = WsEnvelope<FrontendCommandData>;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum FrontendCommandData {
    /// Start charging: send BB01 (connect) then BB02 (setpoints + delta).
    StartCharging {
        /// Charger voltage setpoint in volts.
        #[serde(rename = "chargerVoltage")]
        charger_voltage: f32,
        /// Charger current setpoint in amperes.
        #[serde(rename = "chargerCurrent")]
        charger_current: f32,
        /// Target cell delta in millivolts.
        #[serde(rename = "targetDeltaMv")]
        target_delta_mv: f32,
    },
    /// Stop charging: send BBFF (soft-stop).
    StopCharging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum System {
    Backend,
    Bms,
    Daq,
    Vcu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Device {
    Bms,
    Vcu,
    Raspi,
    NodeFL,
    NodeFR,
    NodeRL,
    NodeRR,
    NodeDash,
    Nucleo1,
    Nucleo2,
    NodePDMDASH,
    NodePDMPCBPanel,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum BackendEventData {
    DeviceRegistrySnapshot { devices: Vec<DeviceStatus> },
    DeviceStatusChanged { device: DeviceStatus },
    DaqTelemetry { telemetry: DaqTelemetry },
    BmsTelemetry { telemetry: BmsTelemetry },
    BackendError { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum DaqTelemetry {
    Temperature {
        source: Device,
        samples: [TemperatureSample; TEMPERATURE_SAMPLE_COUNT],
    },
    Imu {
        source: Device,
        samples: [ImuSample; IMU_SAMPLE_COUNT],
    },
    #[serde(rename = "tbd")]
    Tbd {
        source: Device,
        value: MeasurementValue,
    },
}

pub const TEMPERATURE_SAMPLE_COUNT: usize = 16;
pub const IMU_SAMPLE_COUNT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemperatureSample {
    pub tire: Celsius,
    pub brake: Celsius,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImuSample {
    pub acceleration: Acceleration,
    pub angular_acceleration: AngularAcceleration,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Acceleration {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AngularAcceleration {
    pub rho: f32,
    pub theta: f32,
    pub phi: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum BmsTelemetry {
    Voltages {
        source: Device,
        readings: BmsVoltageReadings,
    },
    /// Pack metadata from BMS_BATTERY_PACK_DATA (0xB000).
    PackData {
        source: Device,
        #[serde(rename = "packVoltage")]
        pack_voltage: Volts,
        #[serde(rename = "stateOfCharge")]
        state_of_charge: Percent,
        /// Pack current in amperes (negative = discharge).
        #[serde(rename = "packCurrent")]
        pack_current: Amperes,
    },
    /// One segment's cell voltages (CAN CellVoltagesPackN). `segment` is 1-6; 24 cells.
    SegmentVoltages {
        source: Device,
        segment: u8,
        cells: Vec<Volts>,
    },
    /// One segment's thermistors. `segment` is 1-6; 20 temperatures.
    SegmentTemperatures {
        source: Device,
        segment: u8,
        temps: Vec<Celsius>,
    },
    Temperatures {
        source: Device,
        readings: BmsTemperatureReadings,
    },
    Balancing {
        source: Device,
        #[serde(rename = "activeCell")]
        active_cell: u8,
        #[serde(rename = "dutyCycle")]
        duty_cycle: Percent,
    },
    Faults {
        source: Device,
        code: u32,
        severity: FaultSeverity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BmsVoltageReadings {
    pub pack: Volts,
    pub min_cell: Volts,
    pub max_cell: Volts,
    pub average_cell: Volts,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BmsTemperatureReadings {
    pub min: Celsius,
    pub max: Celsius,
    pub average: Celsius,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceStatus {
    pub node_id: u8,
    pub name: String,
    pub system: String,
    pub online: bool,
    pub last_seen_ms_ago: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, type = "number")]
#[serde(transparent)]
pub struct Celsius(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, type = "number")]
#[serde(transparent)]
pub struct Volts(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, type = "number")]
#[serde(transparent)]
pub struct Percent(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, type = "number")]
#[serde(transparent)]
pub struct Amperes(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, type = "number")]
#[serde(transparent)]
pub struct MeasurementValue(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, type = "number")]
#[serde(transparent)]
pub struct FaultSeverity(pub f32);

pub fn backend_event(data: BackendEventData) -> BackendEvent {
    WsEnvelope {
        system: System::Backend,
        device: Device::Raspi,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_backend_device_status_snapshot() {
        let event = backend_event(BackendEventData::DeviceRegistrySnapshot {
            devices: vec![DeviceStatus {
                node_id: 0x1C,
                name: "BMS".to_string(),
                system: "bms".to_string(),
                online: true,
                last_seen_ms_ago: Some(10),
                last_error: None,
            }],
        });

        let json = serde_json::to_value(&event).expect("event should serialize");

        assert_eq!(json["system"], "backend");
        assert_eq!(json["device"], "raspi");
        assert_eq!(json["data"]["type"], "deviceRegistrySnapshot");
        assert_eq!(json["data"]["devices"][0]["nodeId"], 0x1C);
        assert_eq!(json["data"]["devices"][0]["online"], true);
    }

    #[test]
    fn serializes_backend_daq_imu_event() {
        let event = backend_event(BackendEventData::DaqTelemetry {
            telemetry: DaqTelemetry::Imu {
                source: Device::NodeFL,
                samples: [ImuSample {
                    acceleration: Acceleration {
                        x: 1.0,
                        y: 2.0,
                        z: 3.0,
                    },
                    angular_acceleration: AngularAcceleration {
                        rho: 4.0,
                        theta: 5.0,
                        phi: 6.0,
                    },
                }; IMU_SAMPLE_COUNT],
            },
        });

        let json = serde_json::to_value(&event).expect("event should serialize");

        assert_eq!(json["data"]["type"], "daqTelemetry");
        assert_eq!(json["data"]["telemetry"]["type"], "imu");
        assert_eq!(json["data"]["telemetry"]["source"], "nodeFL");
        assert_eq!(
            json["data"]["telemetry"]["samples"][0]["acceleration"]["x"],
            1.0
        );
    }

    #[test]
    fn serializes_bms_pack_data() {
        let event = backend_event(BackendEventData::BmsTelemetry {
            telemetry: BmsTelemetry::PackData {
                source: Device::Bms,
                pack_voltage: Volts(532.8),
                state_of_charge: Percent(72.5),
                pack_current: Amperes(-15.0),
            },
        });

        let json = serde_json::to_value(&event).expect("event should serialize");
        let telemetry = &json["data"]["telemetry"];
        assert_eq!(
            telemetry["type"], "packData",
            "unexpected telemetry JSON: {telemetry}"
        );
        assert!(
            (telemetry["packVoltage"].as_f64().unwrap_or(f64::NAN) - 532.8).abs() < 1e-3,
            "unexpected telemetry JSON: {telemetry}"
        );
        assert!(
            (telemetry["stateOfCharge"].as_f64().unwrap_or(f64::NAN) - 72.5).abs() < 1e-3,
            "unexpected telemetry JSON: {telemetry}"
        );
        assert!(
            (telemetry["packCurrent"].as_f64().unwrap_or(f64::NAN) + 15.0).abs() < 1e-3,
            "unexpected telemetry JSON: {telemetry}"
        );
    }

    #[test]
    fn serializes_bms_segment_voltages() {
        let mut cells = vec![Volts(3.7); 24];
        cells[23] = Volts(3.8);
        let event = backend_event(BackendEventData::BmsTelemetry {
            telemetry: BmsTelemetry::SegmentVoltages {
                source: Device::Bms,
                segment: 3,
                cells,
            },
        });

        let json = serde_json::to_value(&event).expect("event should serialize");
        let telemetry = &json["data"]["telemetry"];

        assert_eq!(json["data"]["type"], "bmsTelemetry");
        assert_eq!(telemetry["type"], "segmentVoltages");
        assert_eq!(telemetry["source"], "bms");
        assert_eq!(telemetry["segment"], 3);
        assert_eq!(telemetry["cells"].as_array().unwrap().len(), 24);
        assert!((telemetry["cells"][0].as_f64().unwrap() - 3.7).abs() < 1e-5);
        assert!((telemetry["cells"][23].as_f64().unwrap() - 3.8).abs() < 1e-5);

        let round_trip: BackendEvent =
            serde_json::from_value(json).expect("segmentVoltages should deserialize");
        match round_trip.data {
            BackendEventData::BmsTelemetry {
                telemetry: BmsTelemetry::SegmentVoltages {
                    segment,
                    cells: decoded,
                    ..
                },
            } => {
                assert_eq!(segment, 3);
                assert_eq!(decoded.len(), 24);
                assert!((decoded[0].0 - 3.7).abs() < 1e-5);
                assert!((decoded[23].0 - 3.8).abs() < 1e-5);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn serializes_bms_segment_temperatures() {
        let temps: Vec<Celsius> = (0..20).map(|i| Celsius(25.0 + i as f32)).collect();
        let event = backend_event(BackendEventData::BmsTelemetry {
            telemetry: BmsTelemetry::SegmentTemperatures {
                source: Device::Bms,
                segment: 6,
                temps,
            },
        });

        let json = serde_json::to_value(&event).expect("event should serialize");
        let telemetry = &json["data"]["telemetry"];

        assert_eq!(json["data"]["type"], "bmsTelemetry");
        assert_eq!(telemetry["type"], "segmentTemperatures");
        assert_eq!(telemetry["source"], "bms");
        assert_eq!(telemetry["segment"], 6);
        assert_eq!(telemetry["temps"].as_array().unwrap().len(), 20);
        assert_eq!(telemetry["temps"][0], 25.0);
        assert_eq!(telemetry["temps"][19], 44.0);
    }
}
