import { createBroadcastStorage } from "./broadcastStorage.js";
import { createStorage } from "./createStorage.js";
import { createMemoryStorage } from "./memoryStorage.js";

import type { AbstractStorage } from "../types/storage.js";

const fallbackStorage = createMemoryStorage();
let browserStorage: AbstractStorage | undefined;

function getBrowserStorage(): AbstractStorage {
  if (typeof window === "undefined" || !window.sessionStorage) return fallbackStorage;
  if (browserStorage) return browserStorage;

  browserStorage = createBroadcastStorage({
    channelName: "dango.session",
    hydrate: true,
    mirror: true,
    storage: window.sessionStorage,
  });

  return browserStorage;
}

const lazySessionStorage: AbstractStorage = {
  getItem(key) {
    return getBrowserStorage().getItem(key);
  },
  removeItem(key) {
    return getBrowserStorage().removeItem(key);
  },
  setItem(key, value) {
    return getBrowserStorage().setItem(key, value);
  },
  subscribe(key, listener) {
    return getBrowserStorage().subscribe?.(key, listener) ?? (() => {});
  },
};

export const sessionStorage = createStorage({ storage: lazySessionStorage });
