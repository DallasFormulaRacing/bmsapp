import { Battery, Thermometer, TrendingUp, Activity, Zap, Scale } from 'lucide-react';
import { useDrag, useDrop } from 'react-dnd';
import { useRef } from 'react';

interface OverviewStatsProps {
  totalVoltage: number | null;
  avgTemperature: number | null;
  stateOfCharge: number | null;
  cellDelta: number | null;
  powerStatus: 'charging' | 'discharging' | 'idle';
  balancingActive: boolean;
  totalCurrent: number | null;
  packPower: number | null;
  minVoltage: number | null;
  maxVoltage: number | null;
  cardOrder: number[];
  onReorder: (dragIndex: number, hoverIndex: number) => void;
}

function fmt(value: number | null, digits = 1, suffix = ''): string {
  if (value == null || Number.isNaN(value)) return '—';
  return `${value.toFixed(digits)}${suffix}`;
}

interface StatCardProps {
  id: number;
  index: number;
  icon: React.ElementType;
  label: string;
  value: string | React.ReactNode;
  valueClassName?: string;
  moveCard: (dragIndex: number, hoverIndex: number) => void;
}

const ITEM_TYPE = 'STAT_CARD';

function StatCard({ id, index, icon: Icon, label, value, valueClassName, moveCard }: StatCardProps) {
  const ref = useRef<HTMLDivElement>(null);

  const [{ handlerId }, drop] = useDrop({
    accept: ITEM_TYPE,
    collect(monitor) {
      return {
        handlerId: monitor.getHandlerId(),
      };
    },
    hover(item: { index: number }, monitor) {
      if (!ref.current) {
        return;
      }
      const dragIndex = item.index;
      const hoverIndex = index;

      if (dragIndex === hoverIndex) {
        return;
      }

      const hoverBoundingRect = ref.current?.getBoundingClientRect();
      const hoverMiddleX = (hoverBoundingRect.right - hoverBoundingRect.left) / 2;
      const clientOffset = monitor.getClientOffset();
      const hoverClientX = clientOffset!.x - hoverBoundingRect.left;

      if (dragIndex < hoverIndex && hoverClientX < hoverMiddleX) {
        return;
      }
      if (dragIndex > hoverIndex && hoverClientX > hoverMiddleX) {
        return;
      }

      moveCard(dragIndex, hoverIndex);
      item.index = hoverIndex;
    },
  });

  const [{ isDragging }, drag] = useDrag({
    type: ITEM_TYPE,
    item: () => {
      return { id, index };
    },
    collect: (monitor) => ({
      isDragging: monitor.isDragging(),
    }),
  });

  drag(drop(ref));

  return (
    <div
      ref={ref}
      data-handler-id={handlerId}
      className={`bg-card border border-border rounded-lg p-3 cursor-move backdrop-blur-xl hover:border-primary/30 transition-all duration-300 ${
        isDragging ? 'opacity-50 scale-105' : 'opacity-100'
      }`}
    >
      <div className="flex items-center gap-2 text-muted-foreground mb-1">
        <Icon className="w-4 h-4 text-primary" />
        <span className="text-xs">{label}</span>
      </div>
      <div className={`text-lg ${valueClassName || ''}`}>{value}</div>
    </div>
  );
}

export function DraggableOverviewStats({
  totalVoltage,
  avgTemperature,
  stateOfCharge,
  cellDelta,
  powerStatus,
  balancingActive,
  totalCurrent,
  packPower,
  minVoltage,
  maxVoltage,
  cardOrder,
  onReorder,
}: OverviewStatsProps) {
  const getPowerStatusColor = () => {
    if (powerStatus === 'charging') return 'text-green-500';
    if (powerStatus === 'discharging') return 'text-orange-500';
    return 'text-muted-foreground';
  };

  const voltageRange =
    minVoltage != null && maxVoltage != null
      ? `${minVoltage.toFixed(3)}V - ${maxVoltage.toFixed(3)}V`
      : '—';

  const allCards = [
    { id: 0, icon: Battery, label: 'Total Voltage', value: fmt(totalVoltage, 1, 'V') },
    { id: 1, icon: Thermometer, label: 'Avg Temperature', value: fmt(avgTemperature, 1, '°C') },
    {
      id: 2,
      icon: TrendingUp,
      label: 'State of Charge',
      value: stateOfCharge == null ? '—' : `${stateOfCharge.toFixed(0)}%`,
    },
    {
      id: 3,
      icon: Scale,
      label: 'Cell Delta (ΔV)',
      value: cellDelta == null ? '—' : `${(cellDelta * 1000).toFixed(0)}mV`,
    },
    {
      id: 4,
      icon: Activity,
      label: 'Power Status',
      value: <span className="capitalize">{powerStatus}</span>,
      valueClassName: getPowerStatusColor(),
    },
    {
      id: 5,
      icon: Zap,
      label: 'Cell Balancing',
      value: balancingActive ? 'Active' : 'Inactive',
      valueClassName: balancingActive ? 'text-blue-500' : 'text-muted-foreground',
    },
    { id: 6, icon: Activity, label: 'Pack Current', value: fmt(totalCurrent, 1, 'A') },
    {
      id: 7,
      icon: Zap,
      label: 'Pack Power',
      value: packPower == null ? '—' : `${(packPower / 1000).toFixed(2)}kW`,
    },
    {
      id: 8,
      icon: Battery,
      label: 'Voltage Range',
      value: voltageRange,
    },
  ];

  return (
    <div className="grid grid-cols-3 gap-4">
      {cardOrder.map((cardId, index) => {
        const card = allCards[cardId];
        return (
          <StatCard
            key={card.id}
            id={card.id}
            index={index}
            icon={card.icon}
            label={card.label}
            value={card.value}
            valueClassName={card.valueClassName}
            moveCard={onReorder}
          />
        );
      })}
    </div>
  );
}