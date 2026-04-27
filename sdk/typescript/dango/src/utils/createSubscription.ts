import type { EventEmitter } from "eventemitter3";

import { batchPoller } from "./batchPoller.js";

let nextSubscriptionId = 0;

export type SubscriptionOptions<TData> = {
  /**
   * Start a WS subscription that calls `listener` on each event.
   * Returns an unsubscribe function.
   */
  wsSubscribe: (listener: (data: TData) => void) => () => void;

  /**
   * One-shot HTTP query returning the same data shape as the WS event.
   * When `undefined`, HTTP fallback is disabled (WS-only subscription).
   */
  httpQuery?: (() => Promise<TData>) | undefined;

  /** Polling interval in milliseconds when using HTTP fallback. */
  httpInterval: number;

  /** EventEmitter that fires `"connected"` and `"closed"` events. */
  emitter: EventEmitter;

  /** Returns current WS connection status. */
  getStatus: () => { isConnected: boolean };

  /**
   * Milliseconds to wait after WS disconnect before starting HTTP polling.
   * @default 5000
   */
  fallbackDelay?: number;

  /** Called on HTTP polling errors (silent retry on next interval). */
  onError?: (error: unknown) => void;

  /** When false, disables HTTP polling fallback (WS-only mode). Default: true. */
  polling?: boolean;

  /** When true, uses a shared batch poller instead of per-subscription setInterval. */
  batch?: boolean;
};

export type TransportMode = "ws" | "http-polling" | "reconnecting";

/**
 * Creates a resilient subscription that uses WebSocket when available
 * and transparently falls back to HTTP polling when WS is unavailable.
 *
 * @returns Unsubscribe function that cleans up all resources.
 */
export function createSubscription<TData>(
  options: SubscriptionOptions<TData>,
  listener: (data: TData) => void,
): () => void {
  const {
    wsSubscribe,
    httpQuery: rawHttpQuery,
    httpInterval,
    emitter,
    getStatus,
    fallbackDelay = 5_000,
    onError,
    polling = true,
    batch,
  } = options;

  const httpQuery = polling ? rawHttpQuery : undefined;
  const subscriptionId = `sub_${nextSubscriptionId++}`;

  let wsUnsub: (() => void) | null = null;
  let httpTimer: ReturnType<typeof setInterval> | null = null;
  let fallbackTimer: ReturnType<typeof setTimeout> | null = null;
  let disposed = false;
  let currentMode: TransportMode = "reconnecting";

  const stopWs = () => {
    wsUnsub?.();
    wsUnsub = null;
  };

  const stopHttp = () => {
    if (batch) {
      batchPoller.unregister(subscriptionId);
    }
    if (httpTimer !== null) {
      clearInterval(httpTimer);
      httpTimer = null;
    }
  };

  const clearFallbackTimer = () => {
    if (fallbackTimer !== null) {
      clearTimeout(fallbackTimer);
      fallbackTimer = null;
    }
  };

  const poll = async () => {
    if (disposed || !httpQuery) return;
    try {
      const data = await httpQuery();
      if (!disposed) listener(data);
    } catch (error) {
      onError?.(error);
    }
  };

  const startHttp = () => {
    if (disposed || !httpQuery) return;
    stopWs();
    clearFallbackTimer();
    currentMode = "http-polling";
    emitter.emit("transport-mode", currentMode);
    poll();
    if (batch) {
      batchPoller.register(subscriptionId, poll, httpInterval);
    } else {
      httpTimer = setInterval(poll, httpInterval);
    }
  };

  const startWs = () => {
    if (disposed) return;
    stopHttp();
    clearFallbackTimer();
    currentMode = "ws";
    emitter.emit("transport-mode", currentMode);
    wsUnsub = wsSubscribe(listener);
  };

  const onConnected = () => {
    if (disposed) return;
    startWs();
  };

  const onClosed = () => {
    if (disposed) return;
    currentMode = "reconnecting";
    emitter.emit("transport-mode", currentMode);
    clearFallbackTimer();
    fallbackTimer = setTimeout(() => {
      if (disposed) return;
      if (!getStatus().isConnected) startHttp();
    }, fallbackDelay);
  };

  emitter.on("connected", onConnected);
  emitter.on("closed", onClosed);

  if (getStatus().isConnected) {
    startWs();
  } else if (httpQuery) {
    currentMode = "reconnecting";
    emitter.emit("transport-mode", currentMode);
    fallbackTimer = setTimeout(() => {
      if (disposed) return;
      if (!getStatus().isConnected) startHttp();
    }, fallbackDelay);
  }

  return () => {
    disposed = true;
    stopWs();
    stopHttp();
    clearFallbackTimer();
    emitter.off("connected", onConnected);
    emitter.off("closed", onClosed);
  };
}
