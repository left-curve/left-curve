import { create } from "zustand";
import { persist, type PersistStorage } from "zustand/middleware";
import { useState, useEffect, useCallback } from "react";
import { useConfig } from "./useConfig.js";

import type { Storage } from "../types/storage.js";

interface StorageState<T> {
  value: T;
  setHydrated: (hydrated: boolean) => void;
  setValue: (valOrFunc: T | ((prev: T) => T)) => void;
  version: number;
  _hasHydrated: boolean;
}

const storeCache = new Map<string, any>();

export type UseStorageOptions<T> = {
  enabled?: boolean;
  initialValue?: T | (() => T);
  storage?: Storage;
  version?: number;
  migrations?: Record<number | string, (data: any) => T>;
  sync?: boolean;
};

export function useStorage<T>(
  key: string,
  options: UseStorageOptions<T> = {},
): [T, (valOrFunc: T | ((t: T) => T)) => void, boolean] {
  const [isMounted, setIsMounted] = useState(false);
  useEffect(() => {
    setIsMounted(true);
  }, []);

  const { storage: defaultStorage } = useConfig();
  const [channel] = useState(new BroadcastChannel(`dango.storage.${key}`));

  const {
    enabled = true,
    sync = false,
    initialValue: _initialValue_,
    storage = defaultStorage,
    version = 1,
    migrations = {},
  } = options;

  const initialValue =
    typeof _initialValue_ === "function" ? (_initialValue_ as () => T)() : _initialValue_;

  if (!storeCache.has(key)) {
    const store = create<StorageState<T>>()(
      persist(
        (set, get) => ({
          value: initialValue as T,
          version: version,
          _hasHydrated: false,
          setHydrated: (hydrated: boolean) => set({ _hasHydrated: hydrated }),
          setValue: (valOrFunc) => {
            const currentValue = get().value;
            const newValue =
              typeof valOrFunc === "function"
                ? (valOrFunc as (prev: T) => T)(currentValue)
                : valOrFunc;

            set({ value: newValue });
            return newValue;
          },
        }),
        {
          name: key,
          version: version,
          storage: storage as PersistStorage<StorageState<T>>,
          migrate: (persistedState: any, version: number) => {
            const state = persistedState as StorageState<T>;

            if (migrations["*"]) {
              const migratedValue = migrations["*"](state.value);
              return { ...state, value: migratedValue, version };
            }

            if (migrations[version]) {
              const migratedValue = migrations[version](state.value);
              return { ...state, value: migratedValue, version };
            }

            return state;
          },
          onRehydrateStorage: (state) => () => state.setHydrated(true),
        },
      ),
    );

    storeCache.set(key, store);
  }

  const store = storeCache.get(key)!;

  const value = store((state: StorageState<T>) => state.value);
  const _setValue = store((state: StorageState<T>) => state.setValue);
  const hasHydrated = store((state: StorageState<T>) => state._hasHydrated);

  const setValue = useCallback(
    (valOrFunc: T | ((t: T) => void)) => {
      const newState = _setValue(valOrFunc);
      if (sync) channel.postMessage(newState);
    },
    [storage, channel],
  );

  useEffect(() => {
    if (!sync) return;
    function updateStorage(event: MessageEvent) {
      _setValue(() => event.data);
    }
    channel.addEventListener("message", updateStorage);

    return () => {
      channel.removeEventListener("message", updateStorage);
    };
  }, []);

  if (!isMounted) return [initialValue as T, setValue, true];
  if (!enabled) return [initialValue as T, setValue, false];

  return [value as T, setValue, !hasHydrated];
}
