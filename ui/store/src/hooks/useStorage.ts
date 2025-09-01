import { useCallback, useEffect, useState } from "react";
import { useQuery } from "../query.js";

import type { Dispatch, SetStateAction } from "react";
import type { Storage } from "../types/storage.js";
import { useConfig } from "./useConfig.js";

export type UseStorageOptions<T = undefined> = {
  initialValue?: T | (() => T);
  storage?: Storage;
  version?: number;
  enabled?: boolean;
  migrations?: Record<number, (data: any) => T>;
  sync?: boolean;
};
export function useStorage<T = undefined>(
  key: string,
  options: UseStorageOptions<T> = {},
): [T extends undefined ? null : T, Dispatch<SetStateAction<T>>] {
  const { storage: defaultStorage } = useConfig();
  const [channel] = useState(new BroadcastChannel(`dango.storage.${key}`));

  const {
    enabled = true,
    sync = false,
    initialValue: _initialValue_,
    storage = defaultStorage,
    version: __version__ = 1,
    migrations = {},
  } = options;

  const initialValue = (() => {
    if (typeof _initialValue_ !== "function") return _initialValue_ as T;
    return (_initialValue_ as () => T)();
  })();

  const { data, refetch } = useQuery<T | null, Error, T, string[]>({
    enabled,
    queryKey: ["dango", enabled.toString(), key],
    queryFn: () => {
      const { value } = storage.getItem(key, { value: initialValue! }) as { value: T };

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
        const { value } = storage.getItem(key, { value: initialValue! }) as { value: T };
        return (valOrFunc as (prevState: T) => T)(value);
      })();

      storage.setItem(key, { version: __version__, value: newState });
      if (sync) channel.postMessage(newState);
      refetch();
    },
    [storage, key, refetch, __version__],
  );

  useEffect(() => {
    if (!sync) return;
    function updateStorage(event: MessageEvent) {
      storage.setItem(key, { version: __version__, value: event.data });
      refetch();
    }
    channel.addEventListener("message", updateStorage);

    return () => {
      channel.removeEventListener("message", updateStorage);
    };
  }, []);

  return [data as any, setValue];
}

export default useStorage;
