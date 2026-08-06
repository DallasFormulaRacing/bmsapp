import { Battery, Thermometer, TrendingUp, Activity, Zap, Scale } from 'lucide-react';

interface OverviewStatsProps {
  totalVoltage: number;
  avgTemperature: number;
  stateOfCharge: number;
  cellDelta: number;
  powerStatus: 'charging' | 'discharging' | 'idle';
  balancingActive: boolean;
  totalCurrent: number;
  packPower: number;
  minVoltage: number;
  maxVoltage: number;
}

export function OverviewStats({ 
  totalVoltage, 
  avgTemperature, 
  stateOfCharge, 
  cellDelta,
  powerStatus,
  balancingActive,
  totalCurrent,
  packPower,
  minVoltage,
  maxVoltage
}: OverviewStatsProps) {
  const getPowerStatusColor = () => {
    if (powerStatus === 'charging') return 'text-green-500';
    if (powerStatus === 'discharging') return 'text-orange-500';
    return 'text-muted-foreground';
  };

  return (
    <div className="grid grid-cols-3 gap-4 mb-6">
      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 text-muted-foreground mb-1">
          <Battery className="w-4 h-4" />
          <span className="text-xs">Total Voltage</span>
        </div>
        <div className="text-xl">{totalVoltage.toFixed(1)}V</div>
      </div>
      
      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 text-muted-foreground mb-1">
          <Thermometer className="w-4 h-4" />
          <span className="text-xs">Avg Temperature</span>
        </div>
        <div className="text-xl">{avgTemperature.toFixed(1)}°C</div>
      </div>
      
      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 text-muted-foreground mb-1">
          <TrendingUp className="w-4 h-4" />
          <span className="text-xs">State of Charge</span>
        </div>
        <div className="text-xl">{stateOfCharge}%</div>
      </div>

      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 text-muted-foreground mb-1">
          <Scale className="w-4 h-4" />
          <span className="text-xs">Cell Delta (ΔV)</span>
        </div>
        <div className="text-xl">{(cellDelta * 1000).toFixed(0)}mV</div>
      </div>

      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 text-muted-foreground mb-1">
          <Activity className="w-4 h-4" />
          <span className="text-xs">Power Status</span>
        </div>
        <div className={`text-xl capitalize ${getPowerStatusColor()}`}>
          {powerStatus}
        </div>
      </div>

      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 text-muted-foreground mb-1">
          <Zap className="w-4 h-4" />
          <span className="text-xs">Cell Balancing</span>
        </div>
        <div className={`text-xl ${balancingActive ? 'text-blue-500' : 'text-muted-foreground'}`}>
          {balancingActive ? 'Active' : 'Inactive'}
        </div>
      </div>

      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 text-muted-foreground mb-1">
          <Activity className="w-4 h-4" />
          <span className="text-xs">Pack Current</span>
        </div>
        <div className="text-xl">{totalCurrent.toFixed(1)}A</div>
      </div>

      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 text-muted-foreground mb-1">
          <Zap className="w-4 h-4" />
          <span className="text-xs">Pack Power</span>
        </div>
        <div className="text-xl">{(packPower / 1000).toFixed(2)}kW</div>
      </div>

      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 text-muted-foreground mb-1">
          <Battery className="w-4 h-4" />
          <span className="text-xs">Voltage Range</span>
        </div>
        <div className="text-sm">{minVoltage.toFixed(3)}V - {maxVoltage.toFixed(3)}V</div>
      </div>
    </div>
  );
}
