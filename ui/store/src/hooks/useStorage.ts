import { useCallback } from "react";
import { useQuery } from "../query.js";
import { createStorage } from "../storages/createStorage.js";

import type { Dispatch, SetStateAction } from "react";
import type { AbstractStorage } from "../types/storage.js";

export type UseStorageOptions<T = undefined> = {
  initialValue?: T | (() => T);
  storage?: () => AbstractStorage;
  version?: number;
  enabled?: boolean;
  migrations?: Record<number, (data: any) => T>;
};
export function useStorage<T = undefined>(
  key: string,
  options: UseStorageOptions<T> = {},
): [T extends undefined ? null : T, Dispatch<SetStateAction<T>>] {
  const {
    enabled = true,
    initialValue: _initialValue_,
    storage: _storage_,
    version: __version__ = 1,
    migrations = {},
  } = options;

  const storage = (() => {
    return createStorage({
      key: "dango",
      storage: _storage_
        ? _storage_
        : typeof window !== "undefined" && window.localStorage
          ? () => window.localStorage
          : undefined,
    });
  })();

  const initialValue = (() => {
    if (typeof _initialValue_ !== "function") return _initialValue_ as T;
    return (_initialValue_ as () => T)();
  })();

  const { data, refetch } = useQuery<T | null, Error, T, string[]>({
    enabled,
    queryKey: ["dango", enabled.toString(), key],
    queryFn: () => {
      const { value } = storage.getItem(key) as { value: T };

      return value ?? null;
    },
    initialData: () => {
      if (!enabled) return initialValue ?? null;

      const item = storage.getItem(key, {
        value: initialValue!,
      });

      const { version, value } = item as { version: number; value: T };

      const returnValue = value ?? null;

      if (version === __version__) return returnValue;

      if (!version) {
        storage.setItem(key, {
          version: __version__,
          value: initialValue,
        });
        return returnValue;
      }

      const migration = migrations[version];

      if (!migration) {
        storage.setItem(key, {
          version: __version__,
          value: initialValue,
        });
        return returnValue;
      }

      const migratedValue = migration(value);

      storage.setItem(key, {
        version: __version__,
        value: migratedValue,
      });

      return migratedValue ?? null;
    },
  });

  const setValue = useCallback(
    (valOrFunc: T | ((t: T) => void)) => {
      const newState = (() => {
        if (typeof valOrFunc !== "function") return valOrFunc as T;
        const { value } = storage.getItem(key) as { value: T };
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
