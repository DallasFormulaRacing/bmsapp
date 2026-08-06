import { useEffect, useState } from 'react';
import { Zap, Square } from 'lucide-react';
import type { ChargingParams } from '../../types/bridge';

export type { ChargingParams };

interface ChargingDashboardProps {
  /** Production rule: charging controls require BMS online. */
  bmsOnline?: boolean;
  onStartCharging?: (params: ChargingParams) => boolean | void;
  onStopCharging?: () => boolean | void;
}

export function ChargingDashboard({
  bmsOnline = false,
  onStartCharging,
  onStopCharging,
}: ChargingDashboardProps) {
  const [chargerVoltage, setChargerVoltage] = useState('');
  const [chargerCurrent, setChargerCurrent] = useState('');
  const [targetDelta, setTargetDelta] = useState('');
  const [isCharging, setIsCharging] = useState(false);

  useEffect(() => {
    if (!bmsOnline) {
      setIsCharging(false);
    }
  }, [bmsOnline]);

  const parseParams = (): ChargingParams | null => {
    if (!chargerVoltage.trim() || !chargerCurrent.trim() || !targetDelta.trim()) {
      return null;
    }
    const voltage = Number(chargerVoltage);
    const current = Number(chargerCurrent);
    const delta = Number(targetDelta);
    if (![voltage, current, delta].every((n) => Number.isFinite(n) && n >= 0)) {
      return null;
    }
    return {
      chargerVoltage: voltage,
      chargerCurrent: current,
      targetDeltaMv: delta,
    };
  };

  const handleStart = () => {
    if (!bmsOnline) return;
    const params = parseParams();
    if (!params) return;
    const ok = onStartCharging?.(params);
    // Treat missing handler as local-only success; false means TX failed.
    if (ok === false) return;
    setIsCharging(true);
  };

  const handleStop = () => {
    if (!bmsOnline) return;
    const ok = onStopCharging?.();
    if (ok === false) return;
    setIsCharging(false);
  };

  const inputsDisabled = isCharging || !bmsOnline;

  return (
    <div className="h-full bg-card border border-border rounded-lg backdrop-blur-xl p-3 flex flex-col gap-2.5 min-w-0">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-sm uppercase tracking-wider text-primary/80">Charging Session</h3>
        <span
          className={`text-xs uppercase tracking-wide font-medium ${
            !bmsOnline
              ? 'text-muted-foreground'
              : isCharging
                ? 'text-green-500'
                : 'text-muted-foreground'
          }`}
        >
          {!bmsOnline ? 'BMS offline' : isCharging ? 'Charging' : 'Idle'}
        </span>
      </div>

      {/* Voltage + Current share one row */}
      <div className="grid grid-cols-2 gap-2">
        <label className="min-w-0 flex flex-col gap-1">
          <span className="text-xs uppercase tracking-wide text-muted-foreground">
            Charger Voltage
          </span>
          <div className="relative">
            <input
              type="number"
              inputMode="decimal"
              min={0}
              step="0.1"
              value={chargerVoltage}
              onChange={(e) => setChargerVoltage(e.target.value)}
              disabled={inputsDisabled}
              className="w-full px-2.5 py-2 pr-7 text-base font-mono bg-background border border-border rounded text-foreground focus:outline-none focus:ring-1 focus:ring-primary/50 disabled:opacity-50"
            />
            <span className="absolute right-2.5 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
              V
            </span>
          </div>
        </label>

        <label className="min-w-0 flex flex-col gap-1">
          <span className="text-xs uppercase tracking-wide text-muted-foreground">
            Charger Current
          </span>
          <div className="relative">
            <input
              type="number"
              inputMode="decimal"
              min={0}
              step="0.1"
              value={chargerCurrent}
              onChange={(e) => setChargerCurrent(e.target.value)}
              disabled={inputsDisabled}
              className="w-full px-2.5 py-2 pr-7 text-base font-mono bg-background border border-border rounded text-foreground focus:outline-none focus:ring-1 focus:ring-primary/50 disabled:opacity-50"
            />
            <span className="absolute right-2.5 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
              A
            </span>
          </div>
        </label>
      </div>

      <label className="min-w-0 flex flex-col gap-1">
        <span className="text-xs uppercase tracking-wide text-muted-foreground">
          Target Delta
        </span>
        <div className="relative">
          <input
            type="number"
            inputMode="decimal"
            min={0}
            step="1"
            value={targetDelta}
            onChange={(e) => setTargetDelta(e.target.value)}
            disabled={inputsDisabled}
            className="w-full px-2.5 py-2 pr-9 text-base font-mono bg-background border border-border rounded text-foreground focus:outline-none focus:ring-1 focus:ring-primary/50 disabled:opacity-50"
          />
          <span className="absolute right-2.5 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
            mV
          </span>
        </div>
      </label>

      <div className="grid grid-cols-2 gap-2 mt-auto">
        <button
          type="button"
          onClick={handleStart}
          disabled={!bmsOnline || isCharging || parseParams() == null}
          className="inline-flex items-center justify-center gap-1.5 px-2 py-2.5 rounded text-sm font-medium uppercase tracking-wide bg-green-500/15 border border-green-500/40 text-green-500 hover:bg-green-500/25 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
        >
          <Zap className="w-4 h-4" />
          Start Charging
        </button>
        <button
          type="button"
          onClick={handleStop}
          disabled={!bmsOnline || !isCharging}
          className="inline-flex items-center justify-center gap-1.5 px-2 py-2.5 rounded text-sm font-medium uppercase tracking-wide bg-red-500/15 border border-red-500/40 text-red-500 hover:bg-red-500/25 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
        >
          <Square className="w-4 h-4" />
          Stop Charging
        </button>
      </div>
    </div>
  );
}
