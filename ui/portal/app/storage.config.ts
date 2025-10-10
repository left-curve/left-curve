import type { AbstractStorage } from "@left-curve/store/types";
import { MMKV, Mode } from "react-native-mmkv";

export const storage = new MMKV({
  id: "dango.global",
  mode: Mode.MULTI_PROCESS,
  readOnly: false,
});

export function createMMKVStorage(): AbstractStorage {
  return {
    get length() {
      return storage.size;
    },
    key(index) {
      const keys = [...storage.getAllKeys()];
      return keys[index];
    },
    getItem<T>(key: string): T | null {
      const result = storage.getString(key);
      if (!result) return null;
      return result as T;
    },
    setItem(key: string, data: string): void {
      storage.set(key, data);
    },
    removeItem(key: string): void {
      storage.delete(key);
    },
    clear() {
      storage.clearAll();
    },
  };
}
