import type { State } from "./config";

export type StorageItemMap = {
  state: State;
};

export type AbstractStorage = {
  getItem(key: string): string | null | undefined | Promise<string | null | undefined>;
  setItem(key: string, value: string): void | Promise<void>;
  removeItem(key: string): void | Promise<void>;
};

export type CreateStorageParameters = {
  deserialize?: (<type>(value: string) => type | unknown) | undefined;
  key?: string | undefined;
  serialize?: (<type>(value: type | any) => string) | undefined;
  storage?: AbstractStorage | undefined;
};

export type Storage<
  itemMap extends Record<string, unknown> = Record<string, unknown>,
  // break line
  storageItemMap extends StorageItemMap = StorageItemMap & itemMap,
> = {
  key: string;
  getItem<
    key extends keyof storageItemMap,
    value extends storageItemMap[key],
    defaultValue extends value | null | undefined,
  >(
    key: key,
    defaultValue?: defaultValue | undefined,
  ):
    | (defaultValue extends null ? value | null : value)
    | Promise<defaultValue extends null ? value | null : value>;
  setItem<key extends keyof storageItemMap, value extends storageItemMap[key] | null>(
    key: key,
    value: value,
  ): void | Promise<void>;
  removeItem(key: keyof storageItemMap): void | Promise<void>;
};
