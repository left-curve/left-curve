import { registerLiveResourceDebug, syncLiveResourceDebug } from "./debug.js";

import type {
  LiveResourceCachePolicy,
  LiveResourceDebugEntry,
  LiveResourceEmitOptions,
  LiveResourceSnapshot,
} from "./types.js";

type Listener = () => void;

type LiveResourceStartContext<Snapshot extends LiveResourceSnapshot> = {
  emit: (nextSnapshot: Snapshot, options?: LiveResourceEmitOptions) => void;
  error: (error: unknown) => void;
};

export type CreateLiveResourceParameters<Params, Snapshot extends LiveResourceSnapshot> = {
  name: string;
  cache?: LiveResourceCachePolicy;
  getKey: (params: Params) => string;
  getInitialSnapshot: () => Snapshot;
  start: (params: Params, context: LiveResourceStartContext<Snapshot>) => () => void;
  equal?: (previous: Snapshot, next: Snapshot) => boolean;
};

type LiveResourceEntry<Params, Snapshot extends LiveResourceSnapshot> = {
  key: string;
  params: Params;
  snapshot: Snapshot;
  listeners: Set<Listener>;
  refCount: number;
  stop?: () => void;
  version?: number;
  startCount: number;
  stopCount: number;
  updateCount: number;
};

function normalizeError(error: unknown) {
  if (error instanceof Error) return error;
  if (typeof error === "string") return new Error(error);
  return new Error("Unknown live resource error", { cause: error });
}

export type LiveResource<Params, Snapshot extends LiveResourceSnapshot> = {
  name: string;
  acquire: (params: Params) => () => void;
  subscribe: (params: Params, listener: Listener) => () => void;
  getSnapshot: (params: Params) => Snapshot;
  getKey: (params: Params) => string;
  getInitialSnapshot: () => Snapshot;
  acquireKey: (key: string, params: Params) => () => void;
  subscribeKey: (key: string, listener: Listener, params?: Params) => () => void;
  getSnapshotByKey: (key: string) => Snapshot;
  getDebugState: () => {
    activeKeys: number;
    totalListeners: number;
    totalStarts: number;
    totalStops: number;
    totalUpdates: number;
    entries: LiveResourceDebugEntry[];
  };
};

export function createLiveResource<Params, Snapshot extends LiveResourceSnapshot>(
  parameters: CreateLiveResourceParameters<Params, Snapshot>,
): LiveResource<Params, Snapshot> {
  const {
    name,
    cache = "delete-on-release",
    getKey,
    getInitialSnapshot,
    start,
    equal,
  } = parameters;
  const entries = new Map<string, LiveResourceEntry<Params, Snapshot>>();
  let totalStarts = 0;
  let totalStops = 0;
  let totalUpdates = 0;

  function getOrCreateEntry(key: string, params: Params): LiveResourceEntry<Params, Snapshot> {
    const existing = entries.get(key);
    if (existing) {
      existing.params = params;
      return existing;
    }

    const entry: LiveResourceEntry<Params, Snapshot> = {
      key,
      params,
      snapshot: getInitialSnapshot(),
      listeners: new Set(),
      refCount: 0,
      startCount: 0,
      stopCount: 0,
      updateCount: 0,
    };
    entries.set(key, entry);
    syncLiveResourceDebug();
    return entry;
  }

  function notify(entry: LiveResourceEntry<Params, Snapshot>) {
    for (const listener of entry.listeners) listener();
    syncLiveResourceDebug();
  }

  function setSnapshot(
    entry: LiveResourceEntry<Params, Snapshot>,
    nextSnapshot: Snapshot,
    options?: LiveResourceEmitOptions,
  ) {
    if (entries.get(entry.key) !== entry) return;

    // Transports can deliver late HTTP polls or out-of-order stream events after a fresher emit.
    if (
      options?.version !== undefined &&
      entry.version !== undefined &&
      options.version <= entry.version
    ) {
      return;
    }

    if (options?.version !== undefined) entry.version = options.version;

    if (equal?.(entry.snapshot, nextSnapshot) ?? Object.is(entry.snapshot, nextSnapshot)) {
      syncLiveResourceDebug();
      return;
    }

    entry.snapshot = nextSnapshot;
    entry.updateCount += 1;
    totalUpdates += 1;
    notify(entry);
  }

  function setConnecting(entry: LiveResourceEntry<Params, Snapshot>) {
    // Keep ready snapshots stable across duplicate acquires; only idle/error states need a spinner.
    if (entry.snapshot.status !== "idle" && entry.snapshot.status !== "error") return;
    setSnapshot(entry, { ...entry.snapshot, status: "connecting", error: null } as Snapshot);
  }

  function setError(entry: LiveResourceEntry<Params, Snapshot>, error: unknown) {
    const normalizedError = normalizeError(error);
    if (entries.get(entry.key) !== entry) {
      console.error(`[live-resource:${name}] dropped error after release`, normalizedError);
      return;
    }
    setSnapshot(entry, { ...entry.snapshot, status: "error", error: normalizedError } as Snapshot);
  }

  function startEntry(entry: LiveResourceEntry<Params, Snapshot>) {
    if (entry.stop) return;

    setConnecting(entry);
    entry.startCount += 1;
    totalStarts += 1;
    syncLiveResourceDebug();

    try {
      entry.stop = start(entry.params, {
        emit: (nextSnapshot, options) => setSnapshot(entry, nextSnapshot, options),
        error: (error) => setError(entry, error),
      });
    } catch (error) {
      setError(entry, error);
      entry.stop = undefined;
    }
  }

  function stopEntry(entry: LiveResourceEntry<Params, Snapshot>) {
    if (!entry.stop) return;
    entry.stop();
    entry.stop = undefined;
    entry.stopCount += 1;
    totalStops += 1;
  }

  function acquireKey(key: string, params: Params) {
    const entry = getOrCreateEntry(key, params);
    entry.refCount += 1;
    startEntry(entry);
    syncLiveResourceDebug();

    let released = false;
    return () => {
      if (released) return;
      released = true;

      entry.refCount = Math.max(0, entry.refCount - 1);
      if (entry.refCount === 0) {
        stopEntry(entry);
        // Delete only after the last listener has unsubscribed; React can unsubscribe after release.
        if (entry.listeners.size === 0 && cache === "delete-on-release") entries.delete(key);
      }
      syncLiveResourceDebug();
    };
  }

  function subscribeKey(key: string, listener: Listener, params?: Params) {
    const entry = params ? getOrCreateEntry(key, params) : entries.get(key);
    if (!entry) return () => {};

    entry.listeners.add(listener);
    syncLiveResourceDebug();

    return () => {
      entry.listeners.delete(listener);
      if (entry.refCount === 0 && entry.listeners.size === 0 && cache === "delete-on-release") {
        // Both lifecycle ownership and React subscriptions are gone, so the entry is unreachable.
        entries.delete(key);
      }
      syncLiveResourceDebug();
    };
  }

  function getSnapshotByKey(key: string) {
    return entries.get(key)?.snapshot ?? getInitialSnapshot();
  }

  function getDebugState() {
    const debugEntries = Array.from(entries.values()).map((entry) => ({
      key: entry.key,
      refCount: entry.refCount,
      listenerCount: entry.listeners.size,
      status: entry.snapshot.status,
      version: entry.version,
      startCount: entry.startCount,
      stopCount: entry.stopCount,
      updateCount: entry.updateCount,
    }));

    return {
      activeKeys: debugEntries.filter((entry) => entry.refCount > 0).length,
      totalListeners: debugEntries.reduce((total, entry) => total + entry.listenerCount, 0),
      totalStarts,
      totalStops,
      totalUpdates,
      entries: debugEntries,
    };
  }

  const resource: LiveResource<Params, Snapshot> = {
    name,
    acquire: (params) => acquireKey(getKey(params), params),
    subscribe: (params, listener) => subscribeKey(getKey(params), listener, params),
    getSnapshot: (params) => getSnapshotByKey(getKey(params)),
    getKey,
    getInitialSnapshot,
    acquireKey,
    subscribeKey,
    getSnapshotByKey,
    getDebugState,
  };

  registerLiveResourceDebug(name, getDebugState);

  return resource;
}
