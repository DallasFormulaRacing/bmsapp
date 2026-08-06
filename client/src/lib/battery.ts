import type { BmsTelemetry, DeviceStatus } from '../types/bridge';
import {
  BMS_CELLS_PER_SEGMENT,
  BMS_NODE_ID,
  BMS_SEGMENT_COUNT,
  BMS_THERMISTORS_PER_SEGMENT,
} from '../types/bridge';
import type { LinkStatusSnapshot } from '../types/status';

export const SEGMENT_COUNT = BMS_SEGMENT_COUNT;
export const CELLS_PER_SEGMENT = BMS_CELLS_PER_SEGMENT;
export const THERMISTORS_PER_SEGMENT = BMS_THERMISTORS_PER_SEGMENT;

export interface CellData {
  voltages: number[];
  temperatures: number[];
  balancingCells: number[];
}

export interface BatterySegmentData {
  segmentNumber: number;
  cells: CellData;
}

export interface SegmentStats {
  segmentNumber: number;
  minVoltage: number | null;
  avgVoltage: number | null;
  maxVoltage: number | null;
  voltageDelta: number | null;
  minTemp: number | null;
  avgTemp: number | null;
  maxTemp: number | null;
  tempDelta: number | null;
  imbalanceCount: number;
}

export interface PackTelemetry {
  packVoltage: number | null;
  minCellVoltage: number | null;
  maxCellVoltage: number | null;
  avgCellVoltage: number | null;
  minTemp: number | null;
  maxTemp: number | null;
  avgTemp: number | null;
  balancingActive: boolean;
  activeCell: number | null;
  dutyCycle: number | null;
  stateOfCharge: number | null;
  packCurrent: number | null;
  powerStatus: 'charging' | 'discharging' | 'idle';
}

export function emptySegments(): BatterySegmentData[] {
  return Array.from({ length: SEGMENT_COUNT }, (_, i) => ({
    segmentNumber: i + 1,
    cells: {
      voltages: [],
      temperatures: [],
      balancingCells: [],
    },
  }));
}

export function emptyPackTelemetry(): PackTelemetry {
  return {
    packVoltage: null,
    minCellVoltage: null,
    maxCellVoltage: null,
    avgCellVoltage: null,
    minTemp: null,
    maxTemp: null,
    avgTemp: null,
    balancingActive: false,
    activeCell: null,
    dutyCycle: null,
    stateOfCharge: null,
    packCurrent: null,
    powerStatus: 'idle',
  };
}

export function applyBmsTelemetry(prev: PackTelemetry, telemetry: BmsTelemetry): PackTelemetry {
  switch (telemetry.type) {
    case 'voltages':
      return {
        ...prev,
        packVoltage: telemetry.readings.pack,
        minCellVoltage: telemetry.readings.minCell,
        maxCellVoltage: telemetry.readings.maxCell,
        avgCellVoltage: telemetry.readings.averageCell,
      };
    case 'packData': {
      const current = telemetry.packCurrent;
      const powerStatus: PackTelemetry['powerStatus'] =
        current > 1 ? 'charging' : current < -1 ? 'discharging' : 'idle';
      return {
        ...prev,
        packVoltage: telemetry.packVoltage,
        stateOfCharge: telemetry.stateOfCharge,
        packCurrent: current,
        powerStatus,
      };
    }
    case 'temperatures':
      return {
        ...prev,
        minTemp: telemetry.readings.min,
        maxTemp: telemetry.readings.max,
        avgTemp: telemetry.readings.average,
      };
    case 'balancing':
      return {
        ...prev,
        balancingActive: telemetry.dutyCycle > 0,
        activeCell: telemetry.activeCell,
        dutyCycle: telemetry.dutyCycle,
      };
    case 'segmentVoltages':
    case 'segmentTemperatures':
    case 'faults':
      return prev;
    default:
      return prev;
  }
}

/** Fold segment cell/therm arrays into pack min/avg/max for overview cards. */
export function derivePackStatsFromSegments(
  pack: PackTelemetry,
  segments: BatterySegmentData[],
): PackTelemetry {
  const voltages = segments.flatMap((s) => s.cells.voltages);
  const temperatures = segments.flatMap((s) => s.cells.temperatures);

  const next = { ...pack };
  if (voltages.length > 0) {
    const min = Math.min(...voltages);
    const max = Math.max(...voltages);
    next.minCellVoltage = min;
    next.maxCellVoltage = max;
    next.avgCellVoltage = voltages.reduce((a, b) => a + b, 0) / voltages.length;
  }
  if (temperatures.length > 0) {
    const min = Math.min(...temperatures);
    const max = Math.max(...temperatures);
    next.minTemp = min;
    next.maxTemp = max;
    next.avgTemp = temperatures.reduce((a, b) => a + b, 0) / temperatures.length;
  }
  return next;
}

function withSegmentUpdate(
  segments: BatterySegmentData[],
  segment: number,
  update: (cells: CellData) => CellData,
): BatterySegmentData[] {
  const index = segment - 1;
  if (index < 0 || index >= segments.length) {
    return segments;
  }

  return segments.map((entry, i) =>
    i === index
      ? {
          ...entry,
          cells: update(entry.cells),
        }
      : entry,
  );
}

/** Apply a single segment's cell voltages (segment is 1-based, 24 cells). */
export function applySegmentVoltages(
  segments: BatterySegmentData[],
  telemetry: Extract<BmsTelemetry, { type: 'segmentVoltages' }>,
): BatterySegmentData[] {
  return withSegmentUpdate(segments, telemetry.segment, (cells) => ({
    ...cells,
    voltages: [...telemetry.cells],
  }));
}

/** Apply a single segment's thermistor readings (segment is 1-based, 20 temps). */
export function applySegmentTemperatures(
  segments: BatterySegmentData[],
  telemetry: Extract<BmsTelemetry, { type: 'segmentTemperatures' }>,
): BatterySegmentData[] {
  return withSegmentUpdate(segments, telemetry.segment, (cells) => ({
    ...cells,
    temperatures: [...telemetry.temps],
  }));
}

export function deriveSegmentStats(segments: BatterySegmentData[]): SegmentStats[] {
  return segments.map((segment) => {
    const { voltages, temperatures } = segment.cells;
    const hasV = voltages.length > 0;
    const hasT = temperatures.length > 0;
    const minVoltage = hasV ? Math.min(...voltages) : null;
    const maxVoltage = hasV ? Math.max(...voltages) : null;
    const avgVoltage = hasV ? voltages.reduce((s, v) => s + v, 0) / voltages.length : null;
    const minTemp = hasT ? Math.min(...temperatures) : null;
    const maxTemp = hasT ? Math.max(...temperatures) : null;
    const avgTemp = hasT ? temperatures.reduce((s, t) => s + t, 0) / temperatures.length : null;

    const imbalanceCount =
      avgVoltage == null
        ? 0
        : voltages.filter((v) => Math.abs(v - avgVoltage) > 0.05).length;

    return {
      segmentNumber: segment.segmentNumber,
      minVoltage,
      avgVoltage,
      maxVoltage,
      voltageDelta: minVoltage != null && maxVoltage != null ? maxVoltage - minVoltage : null,
      minTemp,
      avgTemp,
      maxTemp,
      tempDelta: minTemp != null && maxTemp != null ? maxTemp - minTemp : null,
      imbalanceCount,
    };
  });
}

export function buildLinkStatus(args: {
  bridgeConnected: boolean;
  bridgeEndpoint: string;
  bridgeState: 'connected' | 'reconnecting' | 'down';
  latencyMs: number | null;
  lastMessageAgeMs: number | null;
  devices: DeviceStatus[];
}): LinkStatusSnapshot {
  const bms = args.devices.find((d) => d.nodeId === BMS_NODE_ID);

  return {
    bridge: {
      health: args.bridgeConnected ? 'up' : args.bridgeState === 'reconnecting' ? 'degraded' : 'down',
      state: args.bridgeState,
      endpoint: args.bridgeEndpoint,
      latencyMs: args.latencyMs,
      lastMessageAgeMs: args.lastMessageAgeMs,
    },
    can: {
      // In prod can0 stays up with the bridge. Do not couple CAN health to BMS presence.
      health: !args.bridgeConnected
        ? 'unknown'
        : args.bridgeState === 'reconnecting'
          ? 'degraded'
          : 'up',
      interfaceName: 'can0',
      up: args.bridgeConnected && args.bridgeState === 'connected',
      busErrors: 0,
      rxPerSec: 0,
      txPerSec: 0,
    },
    bms: {
      // BMS online = recent CAN frames from NODE_BMS (server 3s silence timeout).
      health: !bms ? 'unknown' : bms.online ? 'up' : 'down',
      lastSeenAgeMs: bms?.lastSeenMsAgo ?? null,
      pingState: !bms ? 'unknown' : bms.online ? 'ok' : 'timeout',
      roundTripMs: null,
      firmware: null,
      nodeId: bms?.name ? `${bms.name} (0x${BMS_NODE_ID.toString(16).toUpperCase()})` : 'NODE_BMS (0x1C)',
    },
  };
}

export function defaultBridgeWsUrl(): string {
  const fromEnv = import.meta.env.VITE_BRIDGE_WS_URL as string | undefined;
  if (fromEnv) return fromEnv;
  const host = typeof window !== 'undefined' ? window.location.hostname : '127.0.0.1';
  return `ws://${host}:9002`;
}
