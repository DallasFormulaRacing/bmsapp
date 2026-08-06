use std::array;
use std::time::Instant;

use crate::can::{
    BmsCanCommand, CanCommand, CanNode, DaqCanCommand, DfrCanId, DfrCanMessageBuf,
};
use crate::device::DeviceRegistry;
use crate::websocket::{
    Acceleration, Amperes, AngularAcceleration, BackendEvent, BackendEventData, BmsTelemetry,
    Celsius, DaqTelemetry, Device, FrontendCommandData, IMU_SAMPLE_COUNT, ImuSample, Percent,
    TEMPERATURE_SAMPLE_COUNT, TemperatureSample, Volts, backend_event,
};

const IMU_PAYLOAD_LEN: usize = 60;
const TEMPERATURE_PAYLOAD_LEN: usize = 64;
const IMU_AXIS_BYTES: usize = 2;
const IMU_AXES_PER_SAMPLE: usize = 3;
const IMU_SAMPLE_BYTES: usize = IMU_AXIS_BYTES * IMU_AXES_PER_SAMPLE;
const IMU_GYRO_OFFSET: usize = 0;
const IMU_ACCEL_OFFSET: usize = IMU_SAMPLE_BYTES * IMU_SAMPLE_COUNT;
const TIRE_TEMPERATURE_OFFSET: usize = 0;
const BRAKE_TEMPERATURE_OFFSET: usize = 2 * TEMPERATURE_SAMPLE_COUNT;
const ACCEL_RAW_TO_G: f32 = 0.122 / 1000.0;
const GYRO_RAW_TO_DPS: f32 = 70.0 / 1000.0;

/// BMS payload sizes / layout (matches firmware gui_drivers and test/bms_emulator.py).
const BMS_CELL_VOLTAGE_PAYLOAD_LEN: usize = 48;
const BMS_CELLS_PER_SEGMENT: usize = 24;
const BMS_THERM_PAYLOAD_LEN: usize = 64;
const BMS_THERM_OFFSET: usize = 4;
const BMS_THERMISTORS_PER_SEGMENT: usize = 20;
const BMS_THERM_SEGMENTS_PER_HALF: usize = 3;
const BMS_PACK_DATA_PAYLOAD_LEN: usize = 24;

pub fn device_status_snapshot(registry: &DeviceRegistry, now: Instant) -> BackendEvent {
    backend_event(BackendEventData::DeviceRegistrySnapshot {
        devices: registry.snapshot(now),
    })
}

pub fn device_status_changed(
    registry: &DeviceRegistry,
    node: CanNode,
    now: Instant,
) -> Option<BackendEvent> {
    let device = registry.get(node)?;
    Some(backend_event(BackendEventData::DeviceStatusChanged {
        device: device.status(now),
    }))
}

/// Build Raspi -> BMS CAN frames for a charging GUI command.
pub fn can_messages_for_frontend_command(
    command: &FrontendCommandData,
) -> Result<Vec<DfrCanMessageBuf>, ChargingEncodeError> {
    match command {
        FrontendCommandData::StartCharging {
            charger_voltage,
            charger_current,
            target_delta_mv,
        } => {
            if !charger_voltage.is_finite()
                || !charger_current.is_finite()
                || !target_delta_mv.is_finite()
                || *charger_voltage < 0.0
                || *charger_current < 0.0
                || *target_delta_mv < 0.0
            {
                return Err(ChargingEncodeError::InvalidSetpoint);
            }

            let volts_deci = float_to_u16_deci(*charger_voltage)?;
            let amps_deci = float_to_u16_deci(*charger_current)?;
            let delta_mv = float_to_u16(*target_delta_mv)?;

            let mut payload = [0u8; 6];
            payload[0..2].copy_from_slice(&volts_deci.to_be_bytes());
            payload[2..4].copy_from_slice(&amps_deci.to_be_bytes());
            payload[4..6].copy_from_slice(&delta_mv.to_be_bytes());

            Ok(vec![
                raspi_to_bms(BmsCanCommand::GuiConnect, Vec::new()),
                raspi_to_bms(BmsCanCommand::GuiChargingRequest, payload.to_vec()),
            ])
        }
        FrontendCommandData::StopCharging => Ok(vec![raspi_to_bms(
            BmsCanCommand::GuiChargingSoftstop,
            Vec::new(),
        )]),
    }
}

fn raspi_to_bms(command: BmsCanCommand, data: Vec<u8>) -> DfrCanMessageBuf {
    DfrCanMessageBuf {
        id: DfrCanId {
            priority: 0,
            target: CanNode::Bms,
            source: CanNode::Raspi,
            command: CanCommand::Bms(command),
        },
        data,
    }
}

/// Raspi -> BMS telemetry poll set (empty payload). Same command IDs as BMS replies.
pub fn bms_telemetry_poll_messages() -> Vec<DfrCanMessageBuf> {
    const COMMANDS: &[BmsCanCommand] = &[
        BmsCanCommand::BatteryPackData,
        BmsCanCommand::CellVoltagesPack1,
        BmsCanCommand::CellVoltagesPack2,
        BmsCanCommand::CellVoltagesPack3,
        BmsCanCommand::CellVoltagesPack4,
        BmsCanCommand::CellVoltagesPack5,
        BmsCanCommand::CellVoltagesPack6,
        BmsCanCommand::SegmentTempsHalf1,
        BmsCanCommand::SegmentTempsHalf2,
    ];
    COMMANDS
        .iter()
        .map(|command| raspi_to_bms(*command, Vec::new()))
        .collect()
}

fn float_to_u16_deci(value: f32) -> Result<u16, ChargingEncodeError> {
    float_to_u16(value * 10.0)
}

fn float_to_u16(value: f32) -> Result<u16, ChargingEncodeError> {
    let rounded = value.round();
    if !(0.0..=u16::MAX as f32).contains(&rounded) {
        return Err(ChargingEncodeError::OutOfRange);
    }
    Ok(rounded as u16)
}

pub fn telemetry_events_for_can_message(
    message: &DfrCanMessageBuf,
) -> Result<Vec<BackendEvent>, TelemetryDecodeError> {
    match message.id.command {
        CanCommand::Daq(DaqCanCommand::ImuData) => {
            let telemetry = decode_imu_telemetry(message)?;
            Ok(vec![backend_event(BackendEventData::DaqTelemetry {
                telemetry,
            })])
        }
        CanCommand::Daq(DaqCanCommand::TempData) => {
            let telemetry = decode_temperature_telemetry(message)?;
            Ok(vec![backend_event(BackendEventData::DaqTelemetry {
                telemetry,
            })])
        }
        CanCommand::Bms(command) => {
            // Request/response: only BMS -> Raspi replies carry telemetry payloads.
            // Ignore Raspi -> BMS polls (and cangw echoes of those polls).
            if message.id.source != CanNode::Bms {
                return Ok(Vec::new());
            }
            decode_bms_telemetry(message, command)
        }
        _ => Ok(Vec::new()),
    }
}

fn decode_bms_telemetry(
    message: &DfrCanMessageBuf,
    command: BmsCanCommand,
) -> Result<Vec<BackendEvent>, TelemetryDecodeError> {
    let source = can_node_to_device(message.id.source)
        .ok_or(TelemetryDecodeError::UnsupportedSource(message.id.source))?;

    match command {
        BmsCanCommand::BatteryPackData => {
            let telemetry = decode_pack_data(message, source)?;
            Ok(vec![bms_event(telemetry)])
        }
        BmsCanCommand::CellVoltagesPack1 => {
            Ok(vec![bms_event(decode_segment_voltages(message, source, 1)?)])
        }
        BmsCanCommand::CellVoltagesPack2 => {
            Ok(vec![bms_event(decode_segment_voltages(message, source, 2)?)])
        }
        BmsCanCommand::CellVoltagesPack3 => {
            Ok(vec![bms_event(decode_segment_voltages(message, source, 3)?)])
        }
        BmsCanCommand::CellVoltagesPack4 => {
            Ok(vec![bms_event(decode_segment_voltages(message, source, 4)?)])
        }
        BmsCanCommand::CellVoltagesPack5 => {
            Ok(vec![bms_event(decode_segment_voltages(message, source, 5)?)])
        }
        BmsCanCommand::CellVoltagesPack6 => {
            Ok(vec![bms_event(decode_segment_voltages(message, source, 6)?)])
        }
        BmsCanCommand::SegmentTempsHalf1 => decode_therm_half(message, source, 1),
        BmsCanCommand::SegmentTempsHalf2 => decode_therm_half(message, source, 4),
        // Other BMS commands are not GUI telemetry yet.
        _ => Ok(Vec::new()),
    }
}

fn bms_event(telemetry: BmsTelemetry) -> BackendEvent {
    backend_event(BackendEventData::BmsTelemetry { telemetry })
}

fn decode_pack_data(
    message: &DfrCanMessageBuf,
    source: Device,
) -> Result<BmsTelemetry, TelemetryDecodeError> {
    require_len(message, "BMS pack data", BMS_PACK_DATA_PAYLOAD_LEN)?;

    let pack_deci = read_u16_be(&message.data, 0);
    let soc_tenths = read_u16_be(&message.data, 2);
    let current_ma = read_i16_be(&message.data, 4);

    Ok(BmsTelemetry::PackData {
        source,
        pack_voltage: Volts(pack_deci as f32 / 10.0),
        state_of_charge: Percent(soc_tenths as f32 / 10.0),
        pack_current: Amperes(current_ma as f32 / 1000.0),
    })
}

fn decode_segment_voltages(
    message: &DfrCanMessageBuf,
    source: Device,
    segment: u8,
) -> Result<BmsTelemetry, TelemetryDecodeError> {
    require_len(message, "BMS cell voltages", BMS_CELL_VOLTAGE_PAYLOAD_LEN)?;

    let mut cells = Vec::with_capacity(BMS_CELLS_PER_SEGMENT);
    for i in 0..BMS_CELLS_PER_SEGMENT {
        let mv = read_i16_be(&message.data, i * 2);
        cells.push(Volts(mv as f32 / 1000.0));
    }

    Ok(BmsTelemetry::SegmentVoltages {
        source,
        segment,
        cells,
    })
}

fn decode_therm_half(
    message: &DfrCanMessageBuf,
    source: Device,
    first_segment: u8,
) -> Result<Vec<BackendEvent>, TelemetryDecodeError> {
    require_len(message, "BMS segment temps", BMS_THERM_PAYLOAD_LEN)?;

    let mut events = Vec::with_capacity(BMS_THERM_SEGMENTS_PER_HALF);
    for seg_offset in 0..BMS_THERM_SEGMENTS_PER_HALF {
        let base = BMS_THERM_OFFSET + seg_offset * BMS_THERMISTORS_PER_SEGMENT;
        let mut temps = Vec::with_capacity(BMS_THERMISTORS_PER_SEGMENT);
        for i in 0..BMS_THERMISTORS_PER_SEGMENT {
            // Firmware sends high byte of aux reading; emulator stores deg C there.
            temps.push(Celsius(message.data[base + i] as f32));
        }
        events.push(bms_event(BmsTelemetry::SegmentTemperatures {
            source,
            segment: first_segment + seg_offset as u8,
            temps,
        }));
    }
    Ok(events)
}

fn require_len(
    message: &DfrCanMessageBuf,
    command: &'static str,
    minimum: usize,
) -> Result<(), TelemetryDecodeError> {
    if message.data.len() < minimum {
        return Err(TelemetryDecodeError::PayloadTooShort {
            command,
            minimum,
            actual: message.data.len(),
        });
    }
    Ok(())
}

fn decode_imu_telemetry(message: &DfrCanMessageBuf) -> Result<DaqTelemetry, TelemetryDecodeError> {
    if message.data.len() < IMU_PAYLOAD_LEN {
        return Err(TelemetryDecodeError::PayloadTooShort {
            command: "DAQ IMU",
            minimum: IMU_PAYLOAD_LEN,
            actual: message.data.len(),
        });
    }

    let source = can_node_to_device(message.id.source)
        .ok_or(TelemetryDecodeError::UnsupportedSource(message.id.source))?;

    let samples = array::from_fn(|index| {
        let gyro = read_imu_axes(&message.data, IMU_GYRO_OFFSET + index * IMU_SAMPLE_BYTES);
        let accel = read_imu_axes(&message.data, IMU_ACCEL_OFFSET + index * IMU_SAMPLE_BYTES);

        ImuSample {
            acceleration: Acceleration {
                x: accel[0] * ACCEL_RAW_TO_G,
                y: accel[1] * ACCEL_RAW_TO_G,
                z: accel[2] * ACCEL_RAW_TO_G,
            },
            angular_acceleration: AngularAcceleration {
                rho: gyro[0] * GYRO_RAW_TO_DPS,
                theta: gyro[1] * GYRO_RAW_TO_DPS,
                phi: gyro[2] * GYRO_RAW_TO_DPS,
            },
        }
    });

    Ok(DaqTelemetry::Imu { source, samples })
}

fn decode_temperature_telemetry(
    message: &DfrCanMessageBuf,
) -> Result<DaqTelemetry, TelemetryDecodeError> {
    if message.data.len() < TEMPERATURE_PAYLOAD_LEN {
        return Err(TelemetryDecodeError::PayloadTooShort {
            command: "DAQ temperature",
            minimum: TEMPERATURE_PAYLOAD_LEN,
            actual: message.data.len(),
        });
    }

    let source = can_node_to_device(message.id.source)
        .ok_or(TelemetryDecodeError::UnsupportedSource(message.id.source))?;

    let samples = array::from_fn(|index| {
        let tire = read_i16_le(&message.data, TIRE_TEMPERATURE_OFFSET + index * 2) as f32 / 10.0;
        let brake = read_i16_le(&message.data, BRAKE_TEMPERATURE_OFFSET + index * 2) as f32 / 10.0;

        TemperatureSample {
            tire: Celsius(tire),
            brake: Celsius(brake),
        }
    });

    Ok(DaqTelemetry::Temperature { source, samples })
}

fn read_imu_axes(data: &[u8], offset: usize) -> [f32; IMU_AXES_PER_SAMPLE] {
    array::from_fn(|axis| {
        let start = offset + axis * IMU_AXIS_BYTES;
        read_i16_le(data, start) as f32
    })
}

fn read_i16_le(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_i16_be(data: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes([data[offset], data[offset + 1]])
}

fn read_u16_be(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([data[offset], data[offset + 1]])
}

fn can_node_to_device(node: CanNode) -> Option<Device> {
    match node {
        CanNode::FrontLeft => Some(Device::NodeFL),
        CanNode::FrontRight => Some(Device::NodeFR),
        CanNode::RearLeft => Some(Device::NodeRL),
        CanNode::RearRight => Some(Device::NodeRR),
        CanNode::Nucleo1 => Some(Device::Nucleo1),
        CanNode::Nucleo2 => Some(Device::Nucleo2),
        CanNode::Vcu => Some(Device::Vcu),
        CanNode::Bms => Some(Device::Bms),
        CanNode::Dash => Some(Device::NodeDash),
        CanNode::Raspi => Some(Device::Raspi),
        CanNode::AllNodes => None,
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum TelemetryDecodeError {
    #[error("{command} payload too short: expected at least {minimum} bytes, got {actual}")]
    PayloadTooShort {
        command: &'static str,
        minimum: usize,
        actual: usize,
    },

    #[error("unsupported telemetry source {0:?}")]
    UnsupportedSource(CanNode),
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ChargingEncodeError {
    #[error("charging setpoint is invalid (NaN/Inf/negative)")]
    InvalidSetpoint,

    #[error("charging setpoint exceeds u16 range")]
    OutOfRange,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::can::{CanCommand, DaqCanCommand, DfrCanId};

    fn daq_message(command: DaqCanCommand, data: Vec<u8>) -> DfrCanMessageBuf {
        DfrCanMessageBuf {
            id: DfrCanId {
                priority: 1,
                target: CanNode::Raspi,
                source: CanNode::Nucleo1,
                command: CanCommand::Daq(command),
            },
            data,
        }
    }

    fn bms_message(command: BmsCanCommand, data: Vec<u8>) -> DfrCanMessageBuf {
        DfrCanMessageBuf {
            id: DfrCanId {
                priority: 0,
                target: CanNode::Raspi,
                source: CanNode::Bms,
                command: CanCommand::Bms(command),
            },
            data,
        }
    }

    fn imu_message(data: Vec<u8>) -> DfrCanMessageBuf {
        daq_message(DaqCanCommand::ImuData, data)
    }

    fn write_i16_le(data: &mut [u8], offset: usize, value: i16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_i16_be(data: &mut [u8], offset: usize, value: i16) {
        data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn write_u16_be(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn decodes_daq_imu_payload_from_firmware_layout() {
        let mut data = vec![0; IMU_PAYLOAD_LEN];

        write_i16_le(&mut data, 0, 100);
        write_i16_le(&mut data, 2, -200);
        write_i16_le(&mut data, 4, 300);
        write_i16_le(&mut data, 30, 1000);
        write_i16_le(&mut data, 32, -2000);
        write_i16_le(&mut data, 34, 3000);

        let events = telemetry_events_for_can_message(&imu_message(data))
            .expect("IMU payload should decode");
        assert_eq!(events.len(), 1);

        let BackendEventData::DaqTelemetry {
            telemetry: DaqTelemetry::Imu { source, samples },
        } = &events[0].data
        else {
            panic!("expected DAQ IMU telemetry event");
        };

        assert_eq!(*source, Device::Nucleo1);
        assert_eq!(samples[0].angular_acceleration.rho, 7.0);
        assert_eq!(samples[0].angular_acceleration.theta, -14.0);
        assert_eq!(samples[0].angular_acceleration.phi, 21.0);
        assert!((samples[0].acceleration.x - 0.122).abs() < f32::EPSILON);
        assert!((samples[0].acceleration.y + 0.244).abs() < f32::EPSILON);
        assert!((samples[0].acceleration.z - 0.366).abs() < f32::EPSILON);
    }

    #[test]
    fn decodes_daq_imu_payload_with_can_fd_padding() {
        let mut data = vec![0; 64];

        write_i16_le(&mut data, 0, 100);
        write_i16_le(&mut data, 30, 1000);
        data[60..64].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

        let events = telemetry_events_for_can_message(&imu_message(data))
            .expect("64-byte CAN FD IMU payload should decode");

        let BackendEventData::DaqTelemetry {
            telemetry: DaqTelemetry::Imu { samples, .. },
        } = &events[0].data
        else {
            panic!("expected DAQ IMU telemetry event");
        };

        assert_eq!(samples[0].angular_acceleration.rho, 7.0);
        assert!((samples[0].acceleration.x - 0.122).abs() < f32::EPSILON);
    }

    #[test]
    fn rejects_wrong_length_imu_payloads() {
        let error = telemetry_events_for_can_message(&imu_message(vec![0; 12]))
            .expect_err("short IMU payload should be rejected");

        assert_eq!(
            error,
            TelemetryDecodeError::PayloadTooShort {
                command: "DAQ IMU",
                minimum: IMU_PAYLOAD_LEN,
                actual: 12,
            }
        );
    }

    #[test]
    fn decodes_daq_temperature_payload_from_firmware_layout() {
        let mut data = vec![0; 64];

        write_i16_le(&mut data, 0, 253);
        write_i16_le(&mut data, 2, -127);
        write_i16_le(&mut data, 32, 405);
        write_i16_le(&mut data, 34, 999);

        let events =
            telemetry_events_for_can_message(&daq_message(DaqCanCommand::TempData, data))
                .expect("temperature payload should decode");

        let BackendEventData::DaqTelemetry {
            telemetry: DaqTelemetry::Temperature { source, samples },
        } = &events[0].data
        else {
            panic!("expected DAQ temperature telemetry event");
        };

        assert_eq!(*source, Device::Nucleo1);
        assert_eq!(samples[0].tire, Celsius(25.3));
        assert_eq!(samples[1].tire, Celsius(-12.7));
        assert_eq!(samples[0].brake, Celsius(40.5));
        assert_eq!(samples[1].brake, Celsius(99.9));
    }

    #[test]
    fn rejects_wrong_length_temperature_payloads() {
        let error = telemetry_events_for_can_message(&daq_message(
            DaqCanCommand::TempData,
            vec![0; TEMPERATURE_PAYLOAD_LEN - 1],
        ))
        .expect_err("short temperature payload should be rejected");

        assert_eq!(
            error,
            TelemetryDecodeError::PayloadTooShort {
                command: "DAQ temperature",
                minimum: TEMPERATURE_PAYLOAD_LEN,
                actual: TEMPERATURE_PAYLOAD_LEN - 1,
            }
        );
    }

    #[test]
    fn device_status_snapshot_uses_backend_envelope() {
        let registry = DeviceRegistry::new();

        let event = device_status_snapshot(&registry, Instant::now());

        assert!(matches!(
            event.data,
            BackendEventData::DeviceRegistrySnapshot { .. }
        ));
    }

    #[test]
    fn decodes_bms_pack_data() {
        let mut data = vec![0; 24];
        write_u16_be(&mut data, 0, 5328); // 532.8 V
        write_u16_be(&mut data, 2, 725); // 72.5 %
        write_i16_be(&mut data, 4, -15000); // -15.0 A

        let events = telemetry_events_for_can_message(&bms_message(
            BmsCanCommand::BatteryPackData,
            data,
        ))
        .expect("pack data should decode");

        let BackendEventData::BmsTelemetry {
            telemetry:
                BmsTelemetry::PackData {
                    source,
                    pack_voltage,
                    state_of_charge,
                    pack_current,
                },
        } = &events[0].data
        else {
            panic!("expected BMS packData telemetry");
        };

        assert_eq!(*source, Device::Bms);
        assert!((pack_voltage.0 - 532.8).abs() < 1e-3);
        assert!((state_of_charge.0 - 72.5).abs() < 1e-3);
        assert!((pack_current.0 + 15.0).abs() < 1e-3);
    }

    #[test]
    fn decodes_bms_segment_voltages() {
        let mut data = vec![0; 48];
        for i in 0..24 {
            write_i16_be(&mut data, i * 2, 3700 + i as i16);
        }

        let events = telemetry_events_for_can_message(&bms_message(
            BmsCanCommand::CellVoltagesPack3,
            data,
        ))
        .expect("cell voltages should decode");

        let BackendEventData::BmsTelemetry {
            telemetry:
                BmsTelemetry::SegmentVoltages {
                    segment,
                    cells,
                    ..
                },
        } = &events[0].data
        else {
            panic!("expected segmentVoltages");
        };

        assert_eq!(*segment, 3);
        assert_eq!(cells.len(), 24);
        assert!((cells[0].0 - 3.7).abs() < 1e-5);
        assert!((cells[23].0 - 3.723).abs() < 1e-5);
    }

    #[test]
    fn encodes_start_charging_connect_then_request() {
        let frames = can_messages_for_frontend_command(&FrontendCommandData::StartCharging {
            charger_voltage: 500.5,
            charger_current: 12.3,
            target_delta_mv: 40.0,
        })
        .expect("setpoints should encode");

        assert_eq!(frames.len(), 2);
        assert_eq!(
            frames[0].id.command,
            CanCommand::Bms(BmsCanCommand::GuiConnect)
        );
        assert!(frames[0].data.is_empty());
        assert_eq!(
            frames[1].id.command,
            CanCommand::Bms(BmsCanCommand::GuiChargingRequest)
        );
        assert_eq!(frames[1].data, vec![0x13, 0x8D, 0x00, 0x7B, 0x00, 0x28]); // 5005, 123, 40
        assert_eq!(frames[1].id.target, CanNode::Bms);
        assert_eq!(frames[1].id.source, CanNode::Raspi);
    }

    #[test]
    fn encodes_stop_charging_softstop() {
        let frames = can_messages_for_frontend_command(&FrontendCommandData::StopCharging)
            .expect("stop should encode");
        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].id.command,
            CanCommand::Bms(BmsCanCommand::GuiChargingSoftstop)
        );
        assert!(frames[0].data.is_empty());
    }

    #[test]
    fn bms_telemetry_polls_are_raspi_to_bms_empty() {
        let frames = bms_telemetry_poll_messages();
        assert_eq!(frames.len(), 9);
        for frame in &frames {
            assert_eq!(frame.id.target, CanNode::Bms);
            assert_eq!(frame.id.source, CanNode::Raspi);
            assert!(frame.data.is_empty());
            assert!(matches!(frame.id.command, CanCommand::Bms(_)));
        }
        assert_eq!(
            frames[0].id.command,
            CanCommand::Bms(BmsCanCommand::BatteryPackData)
        );
        assert_eq!(
            frames[1].id.command,
            CanCommand::Bms(BmsCanCommand::CellVoltagesPack1)
        );
        assert_eq!(
            frames[8].id.command,
            CanCommand::Bms(BmsCanCommand::SegmentTempsHalf2)
        );
    }

    #[test]
    fn ignores_raspi_sourced_bms_poll_echoes() {
        let poll = raspi_to_bms(BmsCanCommand::CellVoltagesPack1, Vec::new());
        let events = telemetry_events_for_can_message(&poll).expect("poll echo is not an error");
        assert!(events.is_empty());
    }

    #[test]
    fn decodes_bms_therm_half_into_three_segments() {
        let mut data = vec![0; 64];
        for i in 0..60 {
            data[4 + i] = 20 + i as u8;
        }

        let events = telemetry_events_for_can_message(&bms_message(
            BmsCanCommand::SegmentTempsHalf1,
            data,
        ))
        .expect("therm half should decode");

        assert_eq!(events.len(), 3);

        for (idx, event) in events.iter().enumerate() {
            let BackendEventData::BmsTelemetry {
                telemetry:
                    BmsTelemetry::SegmentTemperatures {
                        segment,
                        temps,
                        ..
                    },
            } = &event.data
            else {
                panic!("expected segmentTemperatures");
            };
            assert_eq!(*segment, (idx as u8) + 1);
            assert_eq!(temps.len(), 20);
            assert_eq!(temps[0].0, (20 + idx * 20) as f32);
        }
    }
}
