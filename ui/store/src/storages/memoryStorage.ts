import type { AbstractStorage } from "../types/storage.js";

export function createMemoryStorage(): AbstractStorage {
  const store = new Map<string, unknown>();
  return {
    get length() {
      return store.size;
    },
    key(index) {
      const keys = [...store.keys()];
      return keys[index];
    },
    getItem<T>(key: string): T | null {
      const result = store.get(key);
      if (!result) return null;
      return result as T;
    },
    setItem(key: string, data: unknown): void {
      store.set(key, data);
    },
    removeItem(key: string): void {
      store.delete(key);
    },
    clear() {
      store.clear();
    },
  };
}
