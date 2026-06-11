import type { AbstractStorage } from "../types/storage.js";

export function createMemoryStorage(): AbstractStorage {
  const store = new Map<string, unknown>();
  const listeners = new Map<string, Set<(value: string | null) => void>>();

  function notify(key: string, value: string | null) {
    for (const listener of listeners.get(key) ?? []) listener(value);
  }

  return {
    getItem<T>(key: string): T | null {
      const result = store.get(key);
      if (!result) return null;
      return result as T;
    },
    setItem(key: string, data: unknown): void {
      store.set(key, data);
      notify(key, data as string);
    },
    removeItem(key: string): void {
      store.delete(key);
      notify(key, null);
    },
    subscribe(key, listener) {
      const current = listeners.get(key) ?? new Set<(value: string | null) => void>();
      current.add(listener);
      listeners.set(key, current);

      return () => {
        current.delete(listener);
        if (current.size === 0) listeners.delete(key);
      };
    },
  };
}
