import { createMemoryStorage, createStorage } from "@leftcurve/connect-kit";
import { deserializeJson, serializeJson } from "@leftcurve/encoding";
import type { Storage } from "@leftcurve/types";
import { type Dispatch, type SetStateAction, useEffect, useState } from "react";

export type UseStorageOptions<T = undefined> = {
  initialValue?: T | (() => T);
  storage?: Storage;
};
export function useStorage<T = undefined>(
  key: string,
  options: UseStorageOptions<T>,
): [T, Dispatch<SetStateAction<T>>] {
  const { initialValue: _initialValue_, storage: _storage_ } = options;

  const storage = (() => {
    if (_storage_) return _storage_;
    return createStorage({
      deserialize: deserializeJson,
      serialize: serializeJson,
      key: "grustorage",
      storage: createMemoryStorage(),
    });
  })();

  const initialValue = (() => {
    if (typeof _initialValue_ !== "function") return _initialValue_ as T;
    return (_initialValue_ as () => T)();
  })();

  const [value, _setValue] = useState<T>(initialValue);

  // biome-ignore lint/correctness/useExhaustiveDependencies: This effect should only run once
  useEffect(() => {
    (async () => {
      const value = await storage.getItem(key);
      if (value) {
        _setValue(value as T);
        return;
      }
      _setValue(initialValue);
      storage.setItem(key, initialValue);
    })();
  }, []);

  const setValue: Dispatch<SetStateAction<T>> = (valOrFunc) => {
    const newState = (() => {
      if (typeof valOrFunc !== "function") return valOrFunc as T;
      return (valOrFunc as (prevState: T) => T)(value);
    })();
    _setValue(newState);
    storage.setItem(key, newState);
  };

  return [value as T, setValue];
}

export default useStorage;
