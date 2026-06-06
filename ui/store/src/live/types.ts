export type LiveResourceStatus = "idle" | "connecting" | "ready" | "error";

export type LiveResourceSnapshot = {
  status: LiveResourceStatus;
  error: Error | null;
};

export type LiveResourceCachePolicy = "delete-on-release" | "keep";

export type LiveResourceEmitOptions = {
  version?: number;
};

export type LiveResourceDebugEntry = {
  key: string;
  refCount: number;
  listenerCount: number;
  status: LiveResourceStatus;
  version?: number;
  startCount: number;
  stopCount: number;
  updateCount: number;
};

export type LiveResourceDebugState = {
  resources: Record<
    string,
    {
      activeKeys: number;
      totalListeners: number;
      totalStarts: number;
      totalStops: number;
      totalUpdates: number;
      entries: LiveResourceDebugEntry[];
    }
  >;
};
