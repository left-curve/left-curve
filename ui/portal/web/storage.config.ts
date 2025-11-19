import { get, set, del, createStore } from "idb-keyval";

import type { AbstractStorage } from "@left-curve/store/types";

const store = createStore("leftcurve", "dango");

export function createIndexedDBStorage(): AbstractStorage {
  return {
    async getItem<T>(key: string): Promise<T | null> {
      const result = await get<T>(key, store);
      if (!result) return null;
      return result;
    },
    async setItem(key: string, data: string): Promise<void> {
      await set(key, data, store);
    },
    async removeItem(key: string): Promise<void> {
      await del(key, store);
    },
  };
}
