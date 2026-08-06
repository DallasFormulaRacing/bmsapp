import { useMemo } from 'react';
import { useTheme } from 'next-themes';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import {
  toRelativeChartData,
  type AnalyticsPoint,
  type AnalyticsSessionPeaks,
} from '../../hooks/useAnalyticsHistory';
import PartitionBar, {
  PartitionBarSegment,
  PartitionBarSegmentTitle,
  PartitionBarSegmentValue,
} from './ui/partition-bar';

function CollapsibleChart({
  title,
  stamp,
  children,
}: {
  title: string;
  stamp?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="bg-card border border-border rounded-lg backdrop-blur-xl hover:border-primary/30 transition-all duration-300">
      <div className="w-full flex items-center justify-between gap-3 p-4 text-left hover:bg-muted/10 transition-colors rounded-t-lg">
        <h3 className="text-sm font-medium tracking-wide uppercase text-primary/90">{title}</h3>
        {stamp}
      </div>
      {children}
    </div>
  );
}

function RunningHighStamp({ label, value }: { label: string; value: string }) {
  return (
    <div className="text-right shrink-0">
      <div className="text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div className="font-mono text-sm text-orange-500">{value}</div>
    </div>
  );
}

export function AnalyticsPage({
  data,
  peaks,
}: {
  data: AnalyticsPoint[];
  peaks: AnalyticsSessionPeaks;
}) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const chartData = useMemo(() => toRelativeChartData(data), [data]);

  const TARGET_CELL_DELTA = 10;
  const TOTAL_CELLS = 6 * 24;
  const cellsBalanced = 0;
  const cellsLeftToBalance = TOTAL_CELLS;

  const colors = {
    primary: isDark ? 'rgba(255,255,255,0.9)' : 'rgba(0,0,0,0.9)',
    secondary: isDark ? 'rgba(180,180,180,0.9)' : 'rgba(80,80,80,0.9)',
    tertiary: isDark ? 'rgba(140,140,140,0.8)' : 'rgba(120,120,120,0.8)',
    grid: isDark ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)',
    axis: isDark ? 'rgba(255,255,255,0.6)' : 'rgba(0,0,0,0.6)',
    axisLabel: isDark ? 'rgba(255,255,255,0.7)' : 'rgba(0,0,0,0.7)',
    tooltipBg: isDark ? 'rgba(0,0,0,0.9)' : 'rgba(255,255,255,0.95)',
    tooltipBorder: isDark ? 'rgba(255,255,255,0.3)' : 'rgba(0,0,0,0.3)',
  };

  const tooltipStyle = {
    backgroundColor: colors.tooltipBg,
    border: `1px solid ${colors.tooltipBorder}`,
    borderRadius: '8px',
  };

  const tempStamp =
    peaks.tempRunningHigh != null
      ? `${peaks.tempRunningHigh.toFixed(1)}°C`
      : '—';
  const voltageStamp =
    peaks.voltageRunningHigh != null
      ? `${peaks.voltageRunningHigh.toFixed(3)} V`
      : '—';

  return (
    <div>
      <h2 className="mb-4 text-base uppercase tracking-wider text-primary/80">Analytics Dashboard</h2>

      <div className="mb-3 bg-card border border-border rounded-lg backdrop-blur-xl p-4">
        <h3 className="text-sm font-medium tracking-wide uppercase text-primary/90 mb-3">
          Cell Balancing Progress
        </h3>
        <PartitionBar size="md">
          <PartitionBarSegment num={Math.max(cellsBalanced, 0.0001)} variant="success">
            <PartitionBarSegmentTitle>Balanced</PartitionBarSegmentTitle>
            <PartitionBarSegmentValue>{cellsBalanced} cells</PartitionBarSegmentValue>
          </PartitionBarSegment>
          <PartitionBarSegment num={cellsLeftToBalance} variant="warning">
            <PartitionBarSegmentTitle>Left to Balance</PartitionBarSegmentTitle>
            <PartitionBarSegmentValue>{cellsLeftToBalance} cells</PartitionBarSegmentValue>
          </PartitionBarSegment>
        </PartitionBar>
        <p className="mt-2 text-xs text-muted-foreground">
          Target Delta: ±{TARGET_CELL_DELTA} mV • Total cells: {TOTAL_CELLS} • awaiting live data
        </p>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <CollapsibleChart title="State of Charge (SoC) vs Time">
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData} margin={{ top: 15, right: 20, left: 45, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" stroke={colors.grid} />
              <XAxis dataKey="time" stroke={colors.axis} style={{ fontSize: '10px' }} />
              <YAxis
                stroke={colors.axis}
                style={{ fontSize: '10px' }}
                domain={[0, 100]}
                label={{
                  value: 'SoC (%)',
                  angle: -90,
                  position: 'insideLeft',
                  style: { fill: colors.axisLabel, fontSize: '10px', textAnchor: 'middle' },
                }}
              />
              <Tooltip contentStyle={tooltipStyle} />
              <Legend verticalAlign="top" height={30} />
              <Line type="monotone" dataKey="soc" stroke={colors.primary} strokeWidth={2} name="SoC %" />
            </LineChart>
          </ResponsiveContainer>
        </CollapsibleChart>

        <CollapsibleChart title="Cell Voltage Delta vs Time">
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData} margin={{ top: 15, right: 20, left: 45, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" stroke={colors.grid} />
              <XAxis dataKey="time" stroke={colors.axis} style={{ fontSize: '10px' }} />
              <YAxis stroke={colors.axis} style={{ fontSize: '10px' }} />
              <Tooltip contentStyle={tooltipStyle} />
              <Legend verticalAlign="top" height={30} />
              <Line
                type="monotone"
                dataKey="cellDelta"
                stroke={colors.primary}
                strokeWidth={2}
                name="Cell Δ (V)"
              />
            </LineChart>
          </ResponsiveContainer>
        </CollapsibleChart>

        <CollapsibleChart
          title="Temperature vs Time"
          stamp={<RunningHighStamp label="Running High" value={tempStamp} />}
        >
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData} margin={{ top: 15, right: 20, left: 45, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" stroke={colors.grid} />
              <XAxis dataKey="time" stroke={colors.axis} style={{ fontSize: '10px' }} />
              <YAxis stroke={colors.axis} style={{ fontSize: '10px' }} />
              <Tooltip contentStyle={tooltipStyle} />
              <Legend verticalAlign="top" height={30} />
              <Line type="monotone" dataKey="tempHigh" stroke={colors.primary} strokeWidth={2} name="High °C" />
              <Line
                type="monotone"
                dataKey="tempAvg"
                stroke={colors.secondary}
                strokeWidth={2}
                strokeDasharray="5 5"
                name="Avg °C"
              />
            </LineChart>
          </ResponsiveContainer>
        </CollapsibleChart>

        <CollapsibleChart
          title="Cell Voltage vs Time"
          stamp={<RunningHighStamp label="Running High" value={voltageStamp} />}
        >
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData} margin={{ top: 15, right: 20, left: 45, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" stroke={colors.grid} />
              <XAxis dataKey="time" stroke={colors.axis} style={{ fontSize: '10px' }} />
              <YAxis stroke={colors.axis} style={{ fontSize: '10px' }} />
              <Tooltip contentStyle={tooltipStyle} />
              <Legend verticalAlign="top" height={30} />
              <Line type="monotone" dataKey="voltageMax" stroke={colors.primary} strokeWidth={2} name="Max V" />
              <Line
                type="monotone"
                dataKey="voltageAvg"
                stroke={colors.secondary}
                strokeWidth={2}
                strokeDasharray="5 5"
                name="Avg V"
              />
              <Line
                type="monotone"
                dataKey="voltageMin"
                stroke={colors.tertiary}
                strokeWidth={2}
                strokeDasharray="8 4"
                name="Min V"
              />
            </LineChart>
          </ResponsiveContainer>
        </CollapsibleChart>
      </div>

      <div className="mt-3 p-3 bg-muted/20 rounded text-xs text-muted-foreground">
        {data.length === 0
          ? 'Charts are empty until historical telemetry is recorded from the can-bridge.'
          : `Recording ${data.length} sample${data.length === 1 ? '' : 's'} • time is relative to now.`}
      </div>
    </div>
  );
}
