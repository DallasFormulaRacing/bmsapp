export type LinkHealth = 'up' | 'degraded' | 'down' | 'unknown';

export interface BridgeLinkStatus {
  health: LinkHealth;
  state: 'connected' | 'reconnecting' | 'down';
  endpoint: string;
  latencyMs: number | null;
  lastMessageAgeMs: number | null;
}

export interface CanLinkStatus {
  health: LinkHealth;
  interfaceName: string;
  up: boolean;
  busErrors: number;
  rxPerSec: number;
  txPerSec: number;
}

export interface BmsNodeStatus {
  health: LinkHealth;
  lastSeenAgeMs: number | null;
  pingState: 'ok' | 'waiting' | 'timeout' | 'unknown';
  roundTripMs: number | null;
  firmware: string | null;
  nodeId: string;
}

export interface LinkStatusSnapshot {
  bridge: BridgeLinkStatus;
  can: CanLinkStatus;
  bms: BmsNodeStatus;
}
