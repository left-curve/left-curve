import type { AbstractStorage } from "../types/storage.js";

export function isStorageAvailable(storage: () => AbstractStorage): boolean {
  try {
    return !!storage();
  } catch (e) {
    return false;
  }
}
