import type { Address } from "@left-curve/sdk/types";
import type { Key, KeyHash } from "./key.js";

export type User = {
  index: number;
  name: Username;
  keys: Record<KeyHash, Key>;
  accounts: Record<AccountIndex, Address>;
};

export type Username = string;

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
  readonly address: Address;
  readonly index: AccountIndex;
  readonly owner: number;
};

export type AccountDetails = Account & {
  readonly username: Username;
};
