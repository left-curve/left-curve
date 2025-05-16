import { useCallback } from "react";
import { useQuery } from "../query.js";
import { createStorage } from "../storages/createStorage.js";

import type { Dispatch, SetStateAction } from "react";
import type { Storage } from "../types/storage.js";

export type UseStorageOptions<T = undefined> = {
  initialValue?: T | (() => T);
  storage?: Storage;
  version?: number;
};
export function useStorage<T = undefined>(
  key: string,
  options: UseStorageOptions<T> = {},
): [T extends undefined ? null : T, Dispatch<SetStateAction<T>>] {
  const { initialValue: _initialValue_, storage: _storage_, version: __version__ = 1 } = options;

  const storage = (() => {
    if (_storage_) return _storage_;
    return createStorage({
      key: "dango",
      storage:
        typeof window !== "undefined" && window.localStorage ? window.localStorage : undefined,
    });
  })();

  const initialValue = (() => {
    if (typeof _initialValue_ !== "function") return _initialValue_ as T;
    return (_initialValue_ as () => T)();
  })();

  const { data, refetch } = useQuery<T | null, Error, T, string[]>({
    queryKey: ["dango", key],
    queryFn: () => {
      const { value } = storage.getItem(key) as { version: number; value: T };

      const returnValue = value ?? null;
      return returnValue;
    },
    initialData: () => {
      const item = storage.getItem(key, {
        version: __version__,
        value: initialValue!,
      });

      const { version, value } = item as { version: number; value: T };

      if (__version__ > version) {
        storage.setItem(key, {
          version: __version__,
          value: initialValue,
        });
        return value as T;
      }

      const returnValue = value ?? null;
      return returnValue;
    },
  });

  const setValue = useCallback(
    (valOrFunc: T | ((t: T) => void)) => {
      const newState = (() => {
        if (typeof valOrFunc !== "function") return valOrFunc as T;
        const { value } = storage.getItem(key) as { version: number; value: T };
        return (valOrFunc as (prevState: T) => T)(value);
      })();

      storage.setItem(key, { version: __version__, value: newState });
      refetch();
    },
    [storage, key, refetch, __version__],
  );

  return [data as any, setValue];
}

export default useStorage;
