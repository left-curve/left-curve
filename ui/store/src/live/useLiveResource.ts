import { useCallback, useEffect, useMemo, useRef } from "react";
import { useSyncExternalStoreWithSelector } from "use-sync-external-store/with-selector.js";

import type { LiveResource } from "./createLiveResource.js";
import type { LiveResourceSnapshot } from "./types.js";

export type UseLiveResourceParameters<Params, Snapshot extends LiveResourceSnapshot, Selection> = {
  resource: LiveResource<Params, Snapshot>;
  params: Params;
  enabled: boolean;
  selector: (snapshot: Snapshot) => Selection;
  equalityFn?: (previous: Selection, next: Selection) => boolean;
  notifyIntervalMs?: number;
  restartToken?: unknown;
};

function createThrottledListener(listener: () => void, intervalMs: number) {
  let isCancelled = false;
  let timeout: ReturnType<typeof setTimeout> | null = null;
  let lastNotifyAt = 0;

  const cancel = () => {
    isCancelled = true;
    if (timeout) clearTimeout(timeout);
  };

  const call = () => {
    if (isCancelled) return;

    const now = Date.now();
    const elapsedMs = now - lastNotifyAt;

    if (elapsedMs >= intervalMs) {
      if (timeout) {
        clearTimeout(timeout);
        timeout = null;
      }
      lastNotifyAt = now;
      listener();
      return;
    }

    if (!timeout) {
      timeout = setTimeout(() => {
        timeout = null;
        lastNotifyAt = Date.now();
        listener();
      }, intervalMs - elapsedMs);
    }
  };

  return { call, cancel };
}

export function useLiveResource<Params, Snapshot extends LiveResourceSnapshot, Selection>(
  parameters: UseLiveResourceParameters<Params, Snapshot, Selection>,
): Selection {
  const {
    resource,
    params,
    enabled,
    selector,
    equalityFn = Object.is,
    notifyIntervalMs,
    restartToken,
  } = parameters;
  const paramsRef = useRef(params);
  // The effect subscribes by stable key, but start needs the latest params after React commits.
  paramsRef.current = params;

  const key = useMemo(
    () => (enabled ? resource.getKey(params) : null),
    [enabled, params, resource],
  );

  useEffect(() => {
    if (!key) return;
    return resource.acquireKey(key, paramsRef.current);
  }, [key, resource, restartToken]);

  const subscribe = useCallback(
    (listener: () => void) => {
      if (!key) return () => {};
      if (!notifyIntervalMs || notifyIntervalMs <= 0) {
        return resource.subscribeKey(key, listener, paramsRef.current);
      }

      const throttled = createThrottledListener(listener, notifyIntervalMs);
      const unsubscribe = resource.subscribeKey(key, throttled.call, paramsRef.current);

      return () => {
        throttled.cancel();
        unsubscribe();
      };
    },
    [key, resource, restartToken, notifyIntervalMs],
  );

  const getSnapshot = useCallback(
    () => (key ? resource.getSnapshotByKey(key) : resource.getInitialSnapshot()),
    [key, resource, restartToken],
  );

  return useSyncExternalStoreWithSelector(
    subscribe,
    getSnapshot,
    resource.getInitialSnapshot,
    selector,
    equalityFn,
  );
}
