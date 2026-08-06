import type { SegmentStats } from '../../lib/battery';

interface SegmentDetailedStatsProps {
  segments: SegmentStats[];
}

function fmt(value: number | null, digits: number, suffix: string): string {
  if (value == null || Number.isNaN(value)) return '—';
  return `${value.toFixed(digits)}${suffix}`;
}

export function SegmentDetailedStats({ segments }: SegmentDetailedStatsProps) {
  return (
    <div>
      <h2 className="mb-3 text-base uppercase tracking-wider text-primary/80">Segment-Level Details</h2>
      <div className="grid grid-cols-3 gap-3">
        {segments.map((segment) => (
          <div
            key={segment.segmentNumber}
            className="bg-card border border-border rounded-lg p-2 backdrop-blur-xl hover:border-primary/30 transition-all duration-300"
          >
            <h3 className="text-sm mb-1.5 font-medium">Segment {segment.segmentNumber}</h3>

            <div className="grid grid-cols-2 gap-2">
              <div>
                <div className="text-sm text-muted-foreground mb-1">Voltage</div>
                <div className="space-y-0.5 text-sm">
                  <div className="flex justify-between gap-1">
                    <span className="text-muted-foreground">Min:</span>
                    <span className="font-mono">{fmt(segment.minVoltage, 3, 'V')}</span>
                  </div>
                  <div className="flex justify-between gap-1">
                    <span className="text-muted-foreground">Avg:</span>
                    <span className="font-mono">{fmt(segment.avgVoltage, 3, 'V')}</span>
                  </div>
                  <div className="flex justify-between gap-1">
                    <span className="text-muted-foreground">Max:</span>
                    <span className="font-mono">{fmt(segment.maxVoltage, 3, 'V')}</span>
                  </div>
                  <div className="flex justify-between gap-1">
                    <span className="text-muted-foreground">Δ:</span>
                    <span className="font-mono">
                      {segment.voltageDelta == null
                        ? '—'
                        : `${(segment.voltageDelta * 1000).toFixed(0)}mV`}
                    </span>
                  </div>
                </div>
              </div>

              <div>
                <div className="text-sm text-muted-foreground mb-1">Temperature</div>
                <div className="space-y-0.5 text-sm">
                  <div className="flex justify-between gap-1">
                    <span className="text-muted-foreground">Min:</span>
                    <span className="font-mono">{fmt(segment.minTemp, 1, '°C')}</span>
                  </div>
                  <div className="flex justify-between gap-1">
                    <span className="text-muted-foreground">Avg:</span>
                    <span className="font-mono">{fmt(segment.avgTemp, 1, '°C')}</span>
                  </div>
                  <div className="flex justify-between gap-1">
                    <span className="text-muted-foreground">Max:</span>
                    <span className="font-mono">{fmt(segment.maxTemp, 1, '°C')}</span>
                  </div>
                  <div className="flex justify-between gap-1">
                    <span className="text-muted-foreground">Δ:</span>
                    <span className="font-mono">{fmt(segment.tempDelta, 1, '°C')}</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="mt-2 pt-2 border-t border-border">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Imbalance Count:</span>
                <span
                  className={`font-mono ${segment.imbalanceCount > 0 ? 'text-orange-500' : 'text-foreground'}`}
                >
                  {segment.imbalanceCount}
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
