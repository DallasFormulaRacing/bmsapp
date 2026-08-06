import {
  Cable,
  Cpu,
  RefreshCw,
  Radio,
  Wifi,
  WifiOff,
} from 'lucide-react';
import { type ElementType, type ReactNode } from 'react';
import type { LinkHealth, LinkStatusSnapshot } from '../../types/status';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/tooltip';

export type {
  BridgeLinkStatus,
  CanLinkStatus,
  BmsNodeStatus,
  LinkStatusSnapshot,
} from '../../types/status';

interface StatusPageProps {
  status: LinkStatusSnapshot;
  onReconnect?: () => void;
}

function healthColor(health: LinkHealth): string {
  switch (health) {
    case 'up':
      return 'bg-green-500';
    case 'degraded':
      return 'bg-yellow-500';
    case 'down':
      return 'bg-red-500';
    default:
      return 'bg-muted-foreground';
  }
}

function healthText(health: LinkHealth): string {
  switch (health) {
    case 'up':
      return 'text-green-500';
    case 'degraded':
      return 'text-yellow-500';
    case 'down':
      return 'text-red-500';
    default:
      return 'text-muted-foreground';
  }
}

function formatAge(ms: number | null): string {
  if (ms == null) return '—';
  if (ms < 1000) return `${ms} ms ago`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)} s ago`;
  return `${Math.floor(ms / 60_000)} min ago`;
}

function formatLatency(ms: number | null): string {
  if (ms == null) return '—';
  return `${ms} ms`;
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-4 py-1.5 border-b border-border/40 last:border-0">
      <span className="text-xs text-muted-foreground uppercase tracking-wide">{label}</span>
      <span className="text-sm font-mono text-foreground text-right truncate">{value}</span>
    </div>
  );
}

function LinkCard({
  title,
  subtitle,
  health,
  icon: Icon,
  children,
}: {
  title: string;
  subtitle: string;
  health: LinkHealth;
  icon: ElementType;
  children: ReactNode;
}) {
  return (
    <div className="h-full bg-card border border-border rounded-lg backdrop-blur-xl p-4">
      <div className="flex items-start gap-3 mb-3">
        <div
          className={`p-2.5 rounded-full shrink-0 ${
            health === 'up'
              ? 'bg-green-500/15 text-green-500'
              : health === 'degraded'
                ? 'bg-yellow-500/15 text-yellow-500'
                : health === 'down'
                  ? 'bg-red-500/15 text-red-500'
                  : 'bg-muted/30 text-muted-foreground'
          }`}
        >
          <Icon className="w-5 h-5" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className={`inline-block w-2 h-2 rounded-full shrink-0 ${healthColor(health)}`} />
            <h3 className="text-sm font-medium text-foreground uppercase tracking-wide truncate">
              {title}
            </h3>
          </div>
          <p className={`text-xs mt-0.5 ${healthText(health)}`}>{subtitle}</p>
        </div>
      </div>
      <div className="space-y-0.5">{children}</div>
    </div>
  );
}

function StackConnector({
  from,
  to,
  direction,
}: {
  from: LinkHealth;
  to: LinkHealth;
  /** Hover hint for which way information moves across this hop. */
  direction: string;
}) {
  const ok = from === 'up' && to === 'up';
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div
          className="flex items-center justify-center px-1 shrink-0 self-center cursor-help"
          aria-label={direction}
        >
          <div className={`h-px w-6 ${ok ? 'bg-green-500/60' : 'bg-border'}`} />
        </div>
      </TooltipTrigger>
      <TooltipContent side="top">{direction}</TooltipContent>
    </Tooltip>
  );
}

function bridgeSubtitle(state: LinkStatusSnapshot['bridge']['state']): string {
  switch (state) {
    case 'connected':
      return 'WebSocket connected';
    case 'reconnecting':
      return 'Reconnecting…';
    default:
      return 'WebSocket down';
  }
}

function canSubtitle(can: LinkStatusSnapshot['can']): string {
  if (can.health === 'unknown') return 'No CAN status';
  if (!can.up) return `${can.interfaceName} down`;
  if (can.health === 'degraded') return `${can.interfaceName} up · degraded`;
  return `${can.interfaceName} up`;
}

function bmsSubtitle(bms: LinkStatusSnapshot['bms']): string {
  switch (bms.pingState) {
    case 'ok':
      return 'Node online';
    case 'waiting':
      return 'Waiting for pong';
    case 'timeout':
      return 'Node offline';
    default:
      return 'No BMS telemetry yet';
  }
}

export function StatusPage({ status, onReconnect }: StatusPageProps) {
  const connected =
    status.bridge.health === 'up' &&
    status.can.up &&
    (status.bms.health === 'up' || status.bms.health === 'degraded');

  const BridgeIcon =
    status.bridge.state === 'connected'
      ? Wifi
      : status.bridge.state === 'reconnecting'
        ? Radio
        : WifiOff;

  return (
    <div className="w-full">
      <div className="flex items-end justify-between gap-4 mb-6">
        <h2 className="text-base uppercase tracking-wider text-primary/80">Link Status</h2>
        <div className="flex items-center gap-3">
          <div className="px-3 py-1.5 rounded border border-border bg-muted/10 text-xs text-muted-foreground">
            Overall:{' '}
            <span className={`font-medium ${connected ? 'text-green-500' : 'text-muted-foreground'}`}>
              {connected ? 'Live' : 'Not live'}
            </span>
          </div>
          <button
            type="button"
            onClick={() => onReconnect?.()}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded text-xs uppercase tracking-wide border border-border bg-muted/20 text-muted-foreground hover:bg-muted/40 hover:text-foreground transition-colors"
            title="Reconnect WebSocket to can-bridge"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            Reconnect
          </button>
        </div>
      </div>

      <div className="flex items-stretch gap-0 min-w-0">
        <div className="min-w-0 flex-1">
          <LinkCard
            title="UI ↔ Bridge"
            subtitle={bridgeSubtitle(status.bridge.state)}
            health={status.bridge.health}
            icon={BridgeIcon}
          >
            <Metric label="State" value={status.bridge.state} />
            <Metric label="Endpoint" value={status.bridge.endpoint} />
            <Metric label="Latency" value={formatLatency(status.bridge.latencyMs)} />
            <Metric label="Last message" value={formatAge(status.bridge.lastMessageAgeMs)} />
          </LinkCard>
        </div>

        <StackConnector
          from={status.bridge.health}
          to={status.can.health}
          direction="Commands -> CAN · Telemetry ← CAN"
        />

        <div className="min-w-0 flex-1">
          <LinkCard
            title="Bridge ↔ CAN"
            subtitle={canSubtitle(status.can)}
            health={status.can.health}
            icon={Cable}
          >
            <Metric label="Interface" value={status.can.interfaceName} />
            <Metric
              label="Link"
              value={status.can.health === 'unknown' ? '—' : status.can.up ? 'up' : 'down'}
            />
            <Metric label="Bus errors" value={String(status.can.busErrors)} />
            <Metric label="RX rate" value={`${status.can.rxPerSec.toFixed(1)} /s`} />
            <Metric label="TX rate" value={`${status.can.txPerSec.toFixed(1)} /s`} />
          </LinkCard>
        </div>

        <StackConnector
          from={status.can.health}
          to={status.bms.health}
          direction="Commands -> BMS · Telemetry ← BMS"
        />

        <div className="min-w-0 flex-1">
          <LinkCard
            title="BMS Node"
            subtitle={bmsSubtitle(status.bms)}
            health={status.bms.health}
            icon={Cpu}
          >
            <Metric label="Node" value={status.bms.nodeId} />
            <Metric label="Last seen" value={formatAge(status.bms.lastSeenAgeMs)} />
            <Metric label="Ping / pong" value={status.bms.pingState} />
            <Metric label="RTT" value={formatLatency(status.bms.roundTripMs)} />
            <Metric label="Firmware" value={status.bms.firmware ?? '—'} />
          </LinkCard>
        </div>
      </div>
    </div>
  );
}
