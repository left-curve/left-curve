import { deserializeJson, serializeJson } from "@leftcurve/encoding";
import type { CreateStorageParameters, Storage, StorageItemMap } from "@leftcurve/types";
import { createMemoryStorage } from "./memoryStorage";

export function createStorage<
  itemMap extends Record<string, unknown> = Record<string, unknown>,
  storageItemMap extends StorageItemMap = StorageItemMap & itemMap,
>(parameters: CreateStorageParameters): Storage<storageItemMap> {
  const {
    deserialize = deserializeJson,
    key: prefix = "dango",
    serialize = serializeJson,
    storage = createMemoryStorage(),
  } = parameters;

  function unwrap<type>(value: type): type | Promise<type> {
    if (value instanceof Promise) return value.then((x) => x).catch(() => null);
    return value;
  }

  return {
    ...storage,
    key: prefix,
    async getItem(key, defaultValue) {
      const value = storage.getItem(`${prefix}_${key as string}`);
      const unwrapped = await unwrap(value);
      if (unwrapped) return deserialize(unwrapped) ?? null;
      return (defaultValue ?? null) as any;
    },
    async setItem(key, value) {
      const storageKey = `${prefix}_${key as string}`;
      if (value === null) await unwrap(storage.removeItem(storageKey));
      else await unwrap(storage.setItem(storageKey, serialize(value)));
    },
    async removeItem(key) {
      await unwrap(storage.removeItem(`${prefix}_${key as string}`));
    },
  };
}
