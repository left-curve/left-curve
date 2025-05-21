import { deserializeJson, serializeJson } from "@left-curve/dango/encoding";

import { isStorageAvailable } from "./isStorageAvailable.js";
import { createMemoryStorage } from "./memoryStorage.js";

import type { CreateStorageParameters, Storage } from "../types/storage.js";

export function createStorage<inner extends Record<string, unknown> = Record<string, unknown>>(
  parameters: CreateStorageParameters = {},
): Storage<inner> {
  const {
    deserialize = deserializeJson,
    key: prefix = "dango",
    serialize = serializeJson,
    storage: _storage_,
  } = parameters;

  const storage = (() => {
    if (!_storage_) return createMemoryStorage();
    return isStorageAvailable(_storage_) ? _storage_() : createMemoryStorage();
  })();

  return {
    ...storage,
    key: prefix,
    getItem(key, defaultValue) {
      const value = storage.getItem(`${prefix}.${key as string}`);
      if (value) return deserialize(value as string) ?? null;
      return (defaultValue ?? null) as any;
    },
    setItem(key, value) {
      const storageKey = `${prefix}.${key as string}`;
      if (value === null) storage.removeItem(storageKey);
      else storage.setItem(storageKey, serialize(value));
    },
    removeItem(key) {
      storage.removeItem(`${prefix}.${key as string}`);
    },
  };
}

export function createAsyncStorage<inner extends Record<string, unknown> = Record<string, unknown>>(
  parameters: CreateStorageParameters = {},
): Storage<inner> {
  const {
    deserialize = deserializeJson,
    key: prefix = "dango",
    serialize = serializeJson,
  } = parameters;

  const storage = (() => {
    if (parameters.storage) {
      const _storage = parameters.storage();
      if (_storage) return _storage;
    }
    return createMemoryStorage();
  })();

  function unwrap<type>(value: type): type | Promise<type> {
    if (value instanceof Promise) return value.then((x) => x).catch(() => null);
    return value;
  }

  return {
    ...storage,
    key: prefix,
    async getItem(key, defaultValue) {
      const value = storage.getItem(`${prefix}.${key as string}`);
      const unwrapped = await unwrap(value);
      if (unwrapped) return deserialize(unwrapped) ?? null;
      return (defaultValue ?? null) as any;
    },
    async setItem(key, value) {
      const storageKey = `${prefix}.${key as string}`;
      if (value === null) await unwrap(storage.removeItem(storageKey));
      else await unwrap(storage.setItem(storageKey, serialize(value)));
    },
    async removeItem(key) {
      await unwrap(storage.removeItem(`${prefix}.${key as string}`));
    },
  };
}
