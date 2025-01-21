import { deserializeJson, serializeJson } from "@left-curve/encoding";
import type { CreateStorageParameters, Storage } from "@left-curve/types";
import { createMemoryStorage } from "./memoryStorage.js";

export function createStorage<inner extends Record<string, unknown> = Record<string, unknown>>(
  parameters: CreateStorageParameters,
): Storage<inner> {
  const {
    deserialize = deserializeJson,
    key: prefix = "grunnect",
    serialize = serializeJson,
    storage = createMemoryStorage(),
  } = parameters;

  return {
    ...storage,
    key: prefix,
    getItem(key, defaultValue) {
      const value = storage.getItem(`${prefix}_${key as string}`);
      if (value) return deserialize(value as string) ?? null;
      return (defaultValue ?? null) as any;
    },
    setItem(key, value) {
      const storageKey = `${prefix}_${key as string}`;
      if (value === null) storage.removeItem(storageKey);
      else storage.setItem(storageKey, serialize(value));
    },
    removeItem(key) {
      storage.removeItem(`${prefix}_${key as string}`);
    },
  };
}

export function createAsyncStorage<inner extends Record<string, unknown> = Record<string, unknown>>(
  parameters: CreateStorageParameters,
): Storage<inner> {
  const {
    deserialize = deserializeJson,
    key: prefix = "grunnect",
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
