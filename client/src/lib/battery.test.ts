import { describe, expect, it } from 'vitest';
import {
  BMS_CELLS_PER_SEGMENT,
  BMS_SEGMENT_COUNT,
  BMS_THERMISTORS_PER_SEGMENT,
} from '../types/bridge';
import {
  applyBmsTelemetry,
  applySegmentTemperatures,
  applySegmentVoltages,
  buildLinkStatus,
  CELLS_PER_SEGMENT,
  derivePackStatsFromSegments,
  deriveSegmentStats,
  emptyPackTelemetry,
  emptySegments,
  SEGMENT_COUNT,
  THERMISTORS_PER_SEGMENT,
} from './battery';
import { BMS_NODE_ID } from '../types/bridge';

describe('pack layout constants', () => {
  it('is 6 segments × 24 voltages × 20 temperatures', () => {
    expect(BMS_SEGMENT_COUNT).toBe(6);
    expect(BMS_CELLS_PER_SEGMENT).toBe(24);
    expect(BMS_THERMISTORS_PER_SEGMENT).toBe(20);
    expect(SEGMENT_COUNT).toBe(6);
    expect(CELLS_PER_SEGMENT).toBe(24);
    expect(THERMISTORS_PER_SEGMENT).toBe(20);
  });
});

describe('emptySegments', () => {
  it('creates six empty segments numbered 1–6', () => {
    const segments = emptySegments();
    expect(segments).toHaveLength(6);
    expect(segments.map((s) => s.segmentNumber)).toEqual([1, 2, 3, 4, 5, 6]);
    for (const segment of segments) {
      expect(segment.cells.voltages).toEqual([]);
      expect(segment.cells.temperatures).toEqual([]);
    }
  });
});

describe('applySegmentVoltages', () => {
  it('writes 24 cell voltages into the target segment only', () => {
    const cells = Array.from({ length: 24 }, (_, i) => 3.7 + i * 0.001);
    const next = applySegmentVoltages(emptySegments(), {
      type: 'segmentVoltages',
      source: 'bms',
      segment: 2,
      cells,
    });

    expect(next[1].cells.voltages).toEqual(cells);
    expect(next[1].cells.voltages).toHaveLength(24);
    expect(next[0].cells.voltages).toEqual([]);
    expect(next[5].cells.voltages).toEqual([]);
  });

  it('ignores out-of-range segment numbers', () => {
    const segments = emptySegments();
    const next = applySegmentVoltages(segments, {
      type: 'segmentVoltages',
      source: 'bms',
      segment: 7,
      cells: Array(24).fill(3.7),
    });
    expect(next).toBe(segments);
  });
});

describe('applySegmentTemperatures', () => {
  it('writes 20 thermistor readings into the target segment only', () => {
    const temps = Array.from({ length: 20 }, (_, i) => 25 + i);
    const next = applySegmentTemperatures(emptySegments(), {
      type: 'segmentTemperatures',
      source: 'bms',
      segment: 6,
      temps,
    });

    expect(next[5].cells.temperatures).toEqual(temps);
    expect(next[5].cells.temperatures).toHaveLength(20);
    expect(next[0].cells.temperatures).toEqual([]);
  });
});

describe('buildLinkStatus', () => {
  it('keeps CAN up when bridge is connected even if BMS is offline', () => {
    const status = buildLinkStatus({
      bridgeConnected: true,
      bridgeEndpoint: 'ws://berry.local:9002',
      bridgeState: 'connected',
      latencyMs: null,
      lastMessageAgeMs: 100,
      devices: [
        {
          nodeId: BMS_NODE_ID,
          name: 'BMS',
          system: 'bms',
          online: false,
          lastSeenMsAgo: null,
          lastError: null,
        },
      ],
    });
    expect(status.can.health).toBe('up');
    expect(status.can.up).toBe(true);
    expect(status.bms.health).toBe('down');
  });

  it('marks BMS up only when BMS node is online', () => {
    const status = buildLinkStatus({
      bridgeConnected: true,
      bridgeEndpoint: 'ws://berry.local:9002',
      bridgeState: 'connected',
      latencyMs: null,
      lastMessageAgeMs: 50,
      devices: [
        {
          nodeId: BMS_NODE_ID,
          name: 'BMS',
          system: 'bms',
          online: true,
          lastSeenMsAgo: 0,
          lastError: null,
        },
      ],
    });
    expect(status.can.health).toBe('up');
    expect(status.bms.health).toBe('up');
  });
});

describe('deriveSegmentStats', () => {
  it('computes min/avg/max from applied segment arrays', () => {
    let segments = emptySegments();
    segments = applySegmentVoltages(segments, {
      type: 'segmentVoltages',
      source: 'bms',
      segment: 1,
      cells: [3.6, 3.7, 3.8, ...Array(21).fill(3.7)],
    });
    segments = applySegmentTemperatures(segments, {
      type: 'segmentTemperatures',
      source: 'bms',
      segment: 1,
      temps: [20, 25, 30, ...Array(17).fill(25)],
    });

    const stats = deriveSegmentStats(segments)[0];
    expect(stats.minVoltage).toBeCloseTo(3.6);
    expect(stats.maxVoltage).toBeCloseTo(3.8);
    expect(stats.minTemp).toBe(20);
    expect(stats.maxTemp).toBe(30);
  });
});

describe('applyBmsTelemetry packData', () => {
  it('applies pack voltage, SOC, current and power status', () => {
    const next = applyBmsTelemetry(emptyPackTelemetry(), {
      type: 'packData',
      source: 'bms',
      packVoltage: 532.8,
      stateOfCharge: 72.5,
      packCurrent: -15,
    });
    expect(next.packVoltage).toBeCloseTo(532.8);
    expect(next.stateOfCharge).toBeCloseTo(72.5);
    expect(next.packCurrent).toBe(-15);
    expect(next.powerStatus).toBe('discharging');
  });
});

describe('derivePackStatsFromSegments', () => {
  it('folds segment arrays into pack min/avg/max', () => {
    let segments = emptySegments();
    segments = applySegmentVoltages(segments, {
      type: 'segmentVoltages',
      source: 'bms',
      segment: 1,
      cells: [3.6, 3.8, ...Array(22).fill(3.7)],
    });
    segments = applySegmentTemperatures(segments, {
      type: 'segmentTemperatures',
      source: 'bms',
      segment: 1,
      temps: [20, 30, ...Array(18).fill(25)],
    });

    const pack = derivePackStatsFromSegments(emptyPackTelemetry(), segments);
    expect(pack.minCellVoltage).toBeCloseTo(3.6);
    expect(pack.maxCellVoltage).toBeCloseTo(3.8);
    expect(pack.minTemp).toBe(20);
    expect(pack.maxTemp).toBe(30);
  });
});
