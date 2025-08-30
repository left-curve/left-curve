import type { AbstractStorage } from "../types/storage.js";

export function createMemoryStorage(): AbstractStorage {
  const store = new Map<string, unknown>();
  return {
    getItem<T>(key: string): T | undefined {
      return store.get(key) as T;
    },
    setItem(key: string, data: unknown): void {
      store.set(key, data);
    },
    removeItem(key: string): void {
      store.delete(key);
    },
  };
}
