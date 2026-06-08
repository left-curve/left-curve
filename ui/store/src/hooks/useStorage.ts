import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import { useConfig } from "./useConfig.js";

import type { Storage } from "../types/storage.js";

type StorageSetter<T> = (valOrFunc: T | ((prev: T) => T)) => void;
type StorageErrorHandler = (error: unknown) => void;

type PersistedStorageState<T> = {
  value: T;
  version?: number;
};

type PersistedStoragePayload<T> = {
  state?: PersistedStorageState<T>;
  version?: number;
};

type StorageSnapshot<T> = {
  hasHydrated: boolean;
  value: T;
};

type StorageEntryOptions<T> = {
  initialValue: T;
  migrations: Record<number | string, (data: any) => T>;
  onError?: StorageErrorHandler;
  version: number;
};

type StorageEntry<T> = {
  configure: (options: StorageEntryOptions<T>) => void;
  getSnapshot: () => StorageSnapshot<T>;
  hydrate: () => void;
  setValue: (valOrFunc: T | ((prev: T) => T)) => void;
  startSync: (onError?: StorageErrorHandler) => () => void;
  subscribe: (listener: () => void) => () => void;
};

const storageEntries = new WeakMap<Storage, Map<string, StorageEntry<unknown>>>();

export type UseStorageOptions<T> = {
  enabled?: boolean;
  initialValue?: T | (() => T);
  storage?: Storage;
  version?: number;
  migrations?: Record<number | string, (data: any) => T>;
  sync?: boolean;
  onError?: StorageErrorHandler;
};

function isFunction<T>(value: T | ((prev: T) => T)): value is (prev: T) => T {
  return typeof value === "function";
}

function getInitialValue<T>(initialValue: T | (() => T) | undefined): T {
  return typeof initialValue === "function" ? (initialValue as () => T)() : (initialValue as T);
}

function toPersistedPayload<T>(value: T, version: number): PersistedStoragePayload<T> {
  return {
    state: {
      value,
      version,
    },
    version,
  };
}

function getPersistedVersion<T>(payload: PersistedStoragePayload<T>): number {
  return payload.version ?? payload.state?.version ?? 0;
}

function getPersistedValue<T>(
  payload: PersistedStoragePayload<T> | null,
  options: StorageEntryOptions<T>,
): { migrated: boolean; value: T } {
  if (!payload?.state) return { migrated: false, value: options.initialValue };

  const persistedVersion = getPersistedVersion(payload);
  const persistedValue = payload.state.value;

  if (persistedVersion === options.version) return { migrated: false, value: persistedValue };

  const migration = options.migrations["*"] ?? options.migrations[persistedVersion];
  if (!migration) return { migrated: false, value: persistedValue };

  return { migrated: true, value: migration(persistedValue) };
}

function getStorageEntry<T>(
  storage: Storage,
  key: string,
  options: StorageEntryOptions<T>,
): StorageEntry<T> {
  let entries = storageEntries.get(storage);
  if (!entries) {
    entries = new Map();
    storageEntries.set(storage, entries);
  }

  const cached = entries.get(key);
  if (cached) return cached as StorageEntry<T>;

  const entry = createStorageEntry(storage, key, options);
  entries.set(key, entry as StorageEntry<unknown>);
  return entry;
}

function createStorageEntry<T>(
  storage: Storage,
  key: string,
  options: StorageEntryOptions<T>,
): StorageEntry<T> {
  let currentOptions = options;
  let value = options.initialValue;
  let snapshot: StorageSnapshot<T> = { hasHydrated: false, value };
  let revision = 0;
  let hydration: Promise<void> | undefined;
  let syncCount = 0;
  let stopSync: (() => void) | undefined;
  const listeners = new Set<() => void>();
  const syncErrorHandlers = new Set<StorageErrorHandler>();

  function notify() {
    snapshot = { hasHydrated: snapshot.hasHydrated, value };
    for (const listener of listeners) listener();
  }

  function setHydrated(hasHydrated: boolean) {
    if (snapshot.hasHydrated === hasHydrated) return;
    snapshot = { hasHydrated, value };
    for (const listener of listeners) listener();
  }

  function reportSyncError(error: unknown) {
    for (const handler of syncErrorHandlers) handler(error);
  }

  function configure(nextOptions: StorageEntryOptions<T>) {
    currentOptions = nextOptions;
  }

  function persist(nextValue: T, version: number, onError?: StorageErrorHandler) {
    const persisted = storage.setItem(key, toPersistedPayload(nextValue, version));
    void Promise.resolve(persisted).catch((error) => onError?.(error));
  }

  function applyPayload(
    payload: PersistedStoragePayload<T> | null,
    options: StorageEntryOptions<T>,
    persistMigration: boolean,
  ) {
    const result = getPersistedValue(payload, options);
    value = result.value;
    notify();

    if (result.migrated && persistMigration) {
      persist(result.value, options.version, options.onError);
    }
  }

  return {
    configure,
    getSnapshot: () => snapshot,
    hydrate() {
      if (snapshot.hasHydrated || hydration) return;

      const startRevision = revision;

      hydration = Promise.resolve(storage.getItem(key))
        .then((payload) => {
          if (revision === startRevision) {
            applyPayload(payload as PersistedStoragePayload<T> | null, currentOptions, true);
          }
        })
        .catch((error) => currentOptions.onError?.(error))
        .finally(() => {
          hydration = undefined;
          setHydrated(true);
        });
    },
    setValue(valOrFunc) {
      const nextValue = isFunction(valOrFunc) ? valOrFunc(value) : valOrFunc;
      revision += 1;
      value = nextValue;
      notify();
      persist(nextValue, currentOptions.version, currentOptions.onError);
    },
    startSync(onError) {
      if (onError) syncErrorHandlers.add(onError);
      syncCount += 1;

      if (syncCount === 1) {
        try {
          stopSync = storage.subscribe?.(key, (payload) => {
            try {
              revision += 1;
              applyPayload(payload as PersistedStoragePayload<T> | null, currentOptions, false);
              setHydrated(true);
            } catch (error) {
              reportSyncError(error);
            }
          });
        } catch (error) {
          reportSyncError(error);
        }
      }

      return () => {
        syncCount -= 1;
        if (onError) syncErrorHandlers.delete(onError);
        if (syncCount > 0) return;

        stopSync?.();
        stopSync = undefined;
      };
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

export function useStorage<T>(
  key: string,
  options: UseStorageOptions<T> = {},
): [T, StorageSetter<T>, boolean] {
  const [isMounted, setIsMounted] = useState(false);
  useEffect(() => {
    setIsMounted(true);
  }, []);

  const { storage: defaultStorage } = useConfig();

  const {
    enabled = true,
    sync = false,
    initialValue: rawInitialValue,
    storage = defaultStorage,
    version = 1,
    migrations = {},
    onError,
  } = options;

  const initialValue = getInitialValue(rawInitialValue);
  const entry = getStorageEntry<T>(storage, key, {
    initialValue,
    migrations,
    onError,
    version,
  });

  const snapshot = useSyncExternalStore(entry.subscribe, entry.getSnapshot, () => ({
    hasHydrated: true,
    value: initialValue,
  }));

  const setValue = useCallback<StorageSetter<T>>(
    (valOrFunc) => {
      entry.configure({
        initialValue,
        migrations,
        onError,
        version,
      });
      entry.setValue(valOrFunc);
    },
    [entry, initialValue, migrations, onError, version],
  );

  useEffect(() => {
    entry.configure({ initialValue, migrations, onError, version });
    if (!enabled) return;
    entry.hydrate();
  }, [enabled, entry, initialValue, migrations, onError, version]);

  useEffect(() => {
    if (!enabled || !sync) return;
    return entry.startSync(onError);
  }, [enabled, entry, onError, sync]);

  if (!isMounted) return [initialValue, setValue, true];
  if (!enabled) return [initialValue, setValue, false];

  return [snapshot.value, setValue, snapshot.hasHydrated];
}
