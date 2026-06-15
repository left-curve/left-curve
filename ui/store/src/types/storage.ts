import type { JsonValue } from "@left-curve/types";

export type AbstractStorage = {
  getItem(key: string): string | null | undefined | Promise<string | null | undefined>;
  setItem(key: string, value: string): void | Promise<void>;
  removeItem(key: string): void | Promise<void>;
  subscribe?(key: string, listener: (value: string | null) => void): () => void;
};

export type CreateStorageParameters = {
  key?: string;
  storage?: AbstractStorage;
  deserialize?: <type>(value: string) => type | unknown;
  serialize?: <type>(value: type | any) => string;
};

export type Storage<inner extends Record<string, unknown> = Record<string, unknown>> = {
  key: string;
  getItem<key extends keyof inner, value extends inner[key], defaultValue extends JsonValue>(
    key: key,
    defaultValue?: defaultValue,
  ):
    | (defaultValue extends null ? value | null : value)
    | Promise<defaultValue extends null ? value | null : value>;
  setItem<key extends keyof inner, value extends inner[key] | null>(
    key: key,
    value: value,
  ): void | Promise<void>;
  removeItem(key: keyof inner): void | Promise<void>;
  subscribe?<key extends keyof inner, value extends inner[key] | null>(
    key: key,
    listener: (value: value) => void,
  ): () => void;
};
