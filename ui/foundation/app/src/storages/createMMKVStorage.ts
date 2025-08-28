import type { AbstractStorage } from "@left-curve/foundation-shared";
import { MMKV, Mode } from "react-native-mmkv";

export function createMMKVStorage(): AbstractStorage {
  const store = new MMKV({
    id: "dango.global",
    mode: Mode.MULTI_PROCESS,
    readOnly: false,
  });

  return {
    getItem<T>(key: string): T | undefined {
      return store.getString(key) as T;
    },
    setItem(key: string, data: string): void {
      store.set(key, data);
    },
    removeItem(key: string): void {
      store.delete(key);
    },
  };
}
