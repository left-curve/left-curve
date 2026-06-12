import { useCallback, useSyncExternalStore } from "react";

type Listener = () => void;

export type LiveResourceInvalidator = {
  invalidate: (key: string) => void;
  subscribe: (key: string, listener: Listener) => () => void;
  getRevision: (key: string | null | undefined) => number;
};

export function createLiveResourceInvalidator(): LiveResourceInvalidator {
  const revisions = new Map<string, number>();
  const listeners = new Map<string, Set<Listener>>();

  return {
    invalidate: (key) => {
      revisions.set(key, (revisions.get(key) ?? 0) + 1);

      for (const listener of listeners.get(key) ?? []) {
        listener();
      }
    },
    subscribe: (key, listener) => {
      const currentListeners = listeners.get(key) ?? new Set<Listener>();
      currentListeners.add(listener);
      listeners.set(key, currentListeners);

      return () => {
        currentListeners.delete(listener);
        if (currentListeners.size === 0) listeners.delete(key);
      };
    },
    getRevision: (key) => {
      if (!key) return 0;
      return revisions.get(key) ?? 0;
    },
  };
}

export function useLiveResourceInvalidationRevision(
  invalidator: LiveResourceInvalidator,
  key: string | null,
) {
  const subscribe = useCallback(
    (listener: Listener) => (key ? invalidator.subscribe(key, listener) : () => {}),
    [invalidator, key],
  );
  const getSnapshot = useCallback(() => invalidator.getRevision(key), [invalidator, key]);

  return useSyncExternalStore(subscribe, getSnapshot, () => 0);
}
