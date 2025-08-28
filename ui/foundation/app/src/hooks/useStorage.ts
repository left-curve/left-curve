import { createStorage, useStorage as sharedUseStorage } from "@left-curve/foundation-shared";
import { createMMKVStorage } from "../storages/createMMKVStorage";

import type { Dispatch, SetStateAction } from "react";
import type { UseStorageOptions } from "@left-curve/foundation-shared";

export function useStorage<T = undefined>(
  key: string,
  options: UseStorageOptions<T> = {},
): [T extends undefined ? null : T, Dispatch<SetStateAction<T>>] {
  return sharedUseStorage<T>(key, {
    ...options,
    storage: options.storage || createStorage({ storage: createMMKVStorage() }),
  });
}
