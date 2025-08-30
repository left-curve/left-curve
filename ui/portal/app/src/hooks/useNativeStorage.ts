import { createStorage, useStorage } from "@left-curve/foundation";

import type { Dispatch, SetStateAction } from "react";
import type { UseStorageOptions } from "@left-curve/foundation";

import { MMKV, Mode } from "react-native-mmkv";

const store = new MMKV({
  id: "dango.global",
  mode: Mode.MULTI_PROCESS,
  readOnly: false,
});

export function useNativeStorage<T = undefined>(
  key: string,
  options: UseStorageOptions<T> = {},
): [T extends undefined ? null : T, Dispatch<SetStateAction<T>>] {
  return useStorage<T>(key, {
    ...options,
    storage:
      options.storage ||
      createStorage({
        storage: {
          getItem<T>(key: string): T | undefined {
            return store.getString(key) as T;
          },
          setItem(key: string, data: string): void {
            store.set(key, data);
          },
          removeItem(key: string): void {
            store.delete(key);
          },
        },
      }),
  });
}
