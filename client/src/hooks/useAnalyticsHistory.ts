import { useEffect, useRef, useState } from 'react';
import type { PackTelemetry } from '../lib/battery';

export interface AnalyticsPoint {
  /** Epoch ms when the sample was taken. */
  t: number;
  soc?: number;
  cellDelta?: number;
  tempHigh?: number;
  tempAvg?: number;
  voltageMax?: number;
  voltageAvg?: number;
  voltageMin?: number;
}

export interface AnalyticsSessionPeaks {
  tempRunningHigh: number | null;
  voltageRunningHigh: number | null;
}

/** Chart-ready point with categorical relative time labels (`now`, `-5s`, ...). */
export interface AnalyticsChartPoint extends Omit<AnalyticsPoint, 't'> {
  time: string;
}

export interface AnalyticsHistory {
  data: AnalyticsPoint[];
  peaks: AnalyticsSessionPeaks;
}

const SAMPLE_MS = 1000;
const MAX_POINTS = 300;

function hasTelemetry(pack: PackTelemetry): boolean {
  return (
    pack.maxTemp != null ||
    pack.avgTemp != null ||
    pack.maxCellVoltage != null ||
    pack.avgCellVoltage != null ||
    pack.stateOfCharge != null
  );
}

/** Format age seconds for axis/tooltip: `now`, `-5s`, `-60s`. */
export function formatRelativeAge(ageSec: number): string {
  if (ageSec === 0) return 'now';
  return `${ageSec}s`;
}

/** Convert stored samples into points with relative time labels from `nowMs`. */
export function toRelativeChartData(
  data: AnalyticsPoint[],
  nowMs: number = Date.now(),
): AnalyticsChartPoint[] {
  return data.map(({ t, ...rest }) => {
    const ageSec = -Math.max(0, Math.round((nowMs - t) / 1000));
    return {
      ...rest,
      time: formatRelativeAge(ageSec),
    };
  });
}

/** Sample pack telemetry at 1 Hz for Analytics charts; tracks session running highs. */
export function useAnalyticsHistory(pack: PackTelemetry): AnalyticsHistory {
  const [data, setData] = useState<AnalyticsPoint[]>([]);
  const [peaks, setPeaks] = useState<AnalyticsSessionPeaks>({
    tempRunningHigh: null,
    voltageRunningHigh: null,
  });
  const packRef = useRef(pack);
  const runningTempHigh = useRef<number | null>(null);
  const runningVoltageHigh = useRef<number | null>(null);

  packRef.current = pack;

  useEffect(() => {
    const id = window.setInterval(() => {
      const p = packRef.current;

      if (!hasTelemetry(p)) {
        setData((prev) => {
          if (prev.length === 0) return prev;
          runningTempHigh.current = null;
          runningVoltageHigh.current = null;
          setPeaks({ tempRunningHigh: null, voltageRunningHigh: null });
          return [];
        });
        return;
      }

      if (p.maxTemp != null) {
        runningTempHigh.current =
          runningTempHigh.current == null
            ? p.maxTemp
            : Math.max(runningTempHigh.current, p.maxTemp);
      }
      if (p.maxCellVoltage != null) {
        runningVoltageHigh.current =
          runningVoltageHigh.current == null
            ? p.maxCellVoltage
            : Math.max(runningVoltageHigh.current, p.maxCellVoltage);
      }

      setPeaks({
        tempRunningHigh: runningTempHigh.current,
        voltageRunningHigh: runningVoltageHigh.current,
      });

      const point: AnalyticsPoint = { t: Date.now() };
      if (p.stateOfCharge != null) point.soc = p.stateOfCharge;
      if (p.minCellVoltage != null && p.maxCellVoltage != null) {
        point.cellDelta = p.maxCellVoltage - p.minCellVoltage;
      }
      if (p.maxTemp != null) point.tempHigh = p.maxTemp;
      if (p.avgTemp != null) point.tempAvg = p.avgTemp;
      if (p.maxCellVoltage != null) point.voltageMax = p.maxCellVoltage;
      if (p.avgCellVoltage != null) point.voltageAvg = p.avgCellVoltage;
      if (p.minCellVoltage != null) point.voltageMin = p.minCellVoltage;

      setData((prev) => {
        const next = [...prev, point];
        return next.length > MAX_POINTS ? next.slice(-MAX_POINTS) : next;
      });
    }, SAMPLE_MS);

    return () => window.clearInterval(id);
  }, []);

  return { data, peaks };
}
