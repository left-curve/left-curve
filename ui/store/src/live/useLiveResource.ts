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
  restartToken?: unknown;
};

export function useLiveResource<Params, Snapshot extends LiveResourceSnapshot, Selection>(
  parameters: UseLiveResourceParameters<Params, Snapshot, Selection>,
): Selection {
  const { resource, params, enabled, selector, equalityFn = Object.is, restartToken } = parameters;
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
    (listener: () => void) =>
      key ? resource.subscribeKey(key, listener, paramsRef.current) : () => {},
    [key, resource, restartToken],
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
