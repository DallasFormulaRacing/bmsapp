import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  ChargingParams,
  DeviceStatus,
  FrontendCommand,
  FrontendCommandData,
  WsEnvelope,
} from '../types/bridge';
import { BMS_NODE_ID } from '../types/bridge';
import {
  applyBmsTelemetry,
  applySegmentTemperatures,
  applySegmentVoltages,
  buildLinkStatus,
  defaultBridgeWsUrl,
  derivePackStatsFromSegments,
  emptyPackTelemetry,
  emptySegments,
  type PackTelemetry,
  type BatterySegmentData,
} from '../lib/battery';
import type { LinkStatusSnapshot } from '../types/status';

const RECONNECT_INTERVAL_MS = 2000;

export function useBridgeSocket(url: string = defaultBridgeWsUrl()) {
  const [devices, setDevices] = useState<DeviceStatus[]>([]);
  const [connected, setConnected] = useState(false);
  const [reconnecting, setReconnecting] = useState(false);
  const [lastMessageAt, setLastMessageAt] = useState<number | null>(null);
  const [lastError, setLastError] = useState<string | null>(null);
  const [pack, setPack] = useState<PackTelemetry>(() => emptyPackTelemetry());
  const [segments, setSegments] = useState<BatterySegmentData[]>(() => emptySegments());
  const [now, setNow] = useState(() => Date.now());

  const socket = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const shouldReconnect = useRef(true);
  const clearReconnectTimer = () => {
    if (reconnectTimer.current) {
      clearTimeout(reconnectTimer.current);
      reconnectTimer.current = null;
    }
  };

  const connect = useCallback(() => {
    if (
      socket.current?.readyState === WebSocket.OPEN ||
      socket.current?.readyState === WebSocket.CONNECTING
    ) {
      return;
    }

    clearReconnectTimer();
    setReconnecting(true);
    setLastError(null);

    const ws = new WebSocket(url);
    socket.current = ws;

    ws.onopen = () => {
      setConnected(true);
      setReconnecting(false);
      setLastMessageAt(Date.now());
    };

    ws.onmessage = (event) => {
      setLastMessageAt(Date.now());
      let msg: WsEnvelope;
      try {
        msg = JSON.parse(event.data) as WsEnvelope;
      } catch (error) {
        console.error('Bridge websocket sent invalid JSON:', error);
        return;
      }

      const data = msg.data;
      switch (data.type) {
        case 'deviceRegistrySnapshot':
          setDevices(data.devices);
          break;
        case 'deviceStatusChanged':
          setDevices((current) => upsertDevice(current, data.device));
          break;
        case 'bmsTelemetry': {
          const telemetry = data.telemetry;
          if (telemetry.type === 'segmentVoltages') {
            setSegments((prev) => applySegmentVoltages(prev, telemetry));
          } else if (telemetry.type === 'segmentTemperatures') {
            setSegments((prev) => applySegmentTemperatures(prev, telemetry));
          } else {
            setPack((prev) => applyBmsTelemetry(prev, telemetry));
          }
          break;
        }
        case 'backendError':
          setLastError(data.message);
          console.error('Bridge error:', data.message);
          break;
        case 'daqTelemetry':
          break;
      }
    };

    ws.onclose = () => {
      setConnected(false);
      socket.current = null;
      if (shouldReconnect.current) {
        setReconnecting(true);
        reconnectTimer.current = setTimeout(connect, RECONNECT_INTERVAL_MS);
      } else {
        setReconnecting(false);
      }
    };

    ws.onerror = () => {
      setLastError('WebSocket connection error');
      ws.close();
    };
  }, [url]);

  const reconnect = useCallback(() => {
    shouldReconnect.current = true;
    socket.current?.close();
    clearReconnectTimer();
    connect();
  }, [connect]);

  const sendFrontendCommand = useCallback((data: FrontendCommandData): boolean => {
    const ws = socket.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      setLastError('Bridge websocket is not connected');
      return false;
    }
    const envelope: FrontendCommand = {
      system: 'backend',
      device: 'raspi',
      data,
    };
    ws.send(JSON.stringify(envelope));
    return true;
  }, []);

  const startCharging = useCallback(
    (params: ChargingParams): boolean => {
      const online = devices.some((d) => d.nodeId === BMS_NODE_ID && d.online);
      if (!online) {
        setLastError('Cannot start charging: BMS is offline');
        return false;
      }
      return sendFrontendCommand({
        type: 'startCharging',
        chargerVoltage: params.chargerVoltage,
        chargerCurrent: params.chargerCurrent,
        targetDeltaMv: params.targetDeltaMv,
      });
    },
    [devices, sendFrontendCommand],
  );

  const stopCharging = useCallback((): boolean => {
    const online = devices.some((d) => d.nodeId === BMS_NODE_ID && d.online);
    if (!online) {
      setLastError('Cannot stop charging: BMS is offline');
      return false;
    }
    return sendFrontendCommand({ type: 'stopCharging' });
  }, [devices, sendFrontendCommand]);

  useEffect(() => {
    shouldReconnect.current = true;
    connect();
    return () => {
      shouldReconnect.current = false;
      clearReconnectTimer();
      socket.current?.close();
      socket.current = null;
    };
  }, [connect]);

  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(id);
  }, []);

  const bmsOnline = devices.some((d) => d.nodeId === BMS_NODE_ID && d.online);

  // Wipe telemetry when bridge drops or BMS goes offline (avoid stale numbers).
  useEffect(() => {
    if (!connected || !bmsOnline) {
      setPack(emptyPackTelemetry());
      setSegments(emptySegments());
    }
  }, [connected, bmsOnline]);

  // Keep pack overview min/avg/max in sync with per-segment arrays.
  useEffect(() => {
    if (!bmsOnline) return;
    setPack((prev) => derivePackStatsFromSegments(prev, segments));
  }, [segments, bmsOnline]);

  const bridgeState: 'connected' | 'reconnecting' | 'down' = connected
    ? 'connected'
    : reconnecting
      ? 'reconnecting'
      : 'down';

  const lastMessageAgeMs =
    lastMessageAt == null ? null : Math.max(0, now - lastMessageAt);

  const linkStatus: LinkStatusSnapshot = useMemo(
    () =>
      buildLinkStatus({
        bridgeConnected: connected,
        bridgeEndpoint: url,
        bridgeState,
        latencyMs: null,
        lastMessageAgeMs,
        devices,
      }),
    [connected, url, bridgeState, lastMessageAgeMs, devices],
  );

  const overallLive =
    linkStatus.bridge.health === 'up' &&
    linkStatus.can.up &&
    (linkStatus.bms.health === 'up' || linkStatus.bms.health === 'degraded');

  return {
    url,
    connected,
    reconnecting,
    bridgeState,
    devices,
    pack,
    segments,
    setSegments,
    linkStatus,
    overallLive,
    lastError,
    lastMessageAgeMs,
    reconnect,
    startCharging,
    stopCharging,
  };
}

function upsertDevice(devices: DeviceStatus[], changed: DeviceStatus): DeviceStatus[] {
  const exists = devices.some((d) => d.nodeId === changed.nodeId);
  if (!exists) {
    return [...devices, changed].sort((a, b) => a.nodeId - b.nodeId);
  }
  return devices.map((d) => (d.nodeId === changed.nodeId ? changed : d));
}
