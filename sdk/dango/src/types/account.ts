import type { Address } from "@left-curve/sdk/types";
import type { Key, KeyHash } from "./key.js";

export type User = {
  keys: Record<KeyHash, Key>;
  accounts: Record<Address, AccountInfo>;
};

export type Username = string;

export type UserIndexAndName = { index: number; name?: string };

export type UserIndexOrName = { index: number } | { name: string };

export type UserStatus = (typeof UserState)[keyof typeof UserState];

export const UserState = {
  Active: "active",
  Inactive: "inactive",
  Frozen: "frozen",
} as const;

export type AccountIndex = number;

export type AccountInfo = {
  readonly index: AccountIndex;
  readonly owner: number;
};

export type Account = {
  readonly username: Username;
  readonly address: Address;
  readonly index: AccountIndex;
  readonly owner: number;
};
