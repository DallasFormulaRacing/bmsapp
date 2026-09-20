/** Wire types aligned with pie can-bridge WebSocket API (camelCase JSON). */

/** Pack layout: 6 segments x 24 cell voltages x 20 thermistors. */
export const BMS_SEGMENT_COUNT = 6;
export const BMS_CELLS_PER_SEGMENT = 24;
export const BMS_THERMISTORS_PER_SEGMENT = 20;

export type System = 'backend' | 'bms' | 'daq' | 'vcu';

export type Device =
  | 'bms'
  | 'vcu'
  | 'raspi'
  | 'nodeFL'
  | 'nodeFR'
  | 'nodeRL'
  | 'nodeRR'
  | 'nodeDash'
  | 'nucleo1'
  | 'nucleo2'
  | 'nodePDMDASH'
  | 'nodePDMPCBPanel';

export interface DeviceStatus {
  nodeId: number;
  name: string;
  system: string;
  online: boolean;
  lastSeenMsAgo: number | null;
  lastError: string | null;
}

export interface BmsVoltageReadings {
  pack: number;
  minCell: number;
  maxCell: number;
  averageCell: number;
}

export interface BmsTemperatureReadings {
  min: number;
  max: number;
  average: number;
}

export type BmsTelemetry =
  | { type: 'voltages'; source: Device; readings: BmsVoltageReadings }
  /** Pack metadata from BMS_BATTERY_PACK_DATA (0xB000). */
  | {
      type: 'packData';
      source: Device;
      packVoltage: number;
      stateOfCharge: number;
      /** Amperes; negative = discharge. */
      packCurrent: number;
    }
  /** Segment 1-6; `cells` length = BMS_CELLS_PER_SEGMENT (24). */
  | { type: 'segmentVoltages'; source: Device; segment: number; cells: number[] }
  /** Segment 1-6; `temps` length = BMS_THERMISTORS_PER_SEGMENT (20). */
  | { type: 'segmentTemperatures'; source: Device; segment: number; temps: number[] }
  | { type: 'temperatures'; source: Device; readings: BmsTemperatureReadings }
  | { type: 'balancing'; source: Device; activeCell: number; dutyCycle: number }
  | { type: 'faults'; source: Device; code: number; severity: number };

export type BackendEventData =
  | { type: 'deviceRegistrySnapshot'; devices: DeviceStatus[] }
  | { type: 'deviceStatusChanged'; device: DeviceStatus }
  | { type: 'daqTelemetry'; telemetry: unknown }
  | { type: 'bmsTelemetry'; telemetry: BmsTelemetry }
  | { type: 'backendError'; message: string };

/** GUI -> can-bridge charging commands. */
export type FrontendCommandData =
  | {
      type: 'startCharging';
      chargerVoltage: number;
      chargerCurrent: number;
      targetDeltaMv: number;
    }
  | { type: 'stopCharging' };

export interface WsEnvelope<T = BackendEventData> {
  system: System;
  device: Device;
  data: T;
}

export type FrontendCommand = WsEnvelope<FrontendCommandData>;

export const BMS_NODE_ID = 0x1f;

/** Charging session setpoints from the Overview charging card. */
export interface ChargingParams {
  chargerVoltage: number;
  chargerCurrent: number;
  targetDeltaMv: number;
}
