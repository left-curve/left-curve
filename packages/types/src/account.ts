import type { Address } from "./address";
import type { Key, KeyHash } from "./key";
import type { Safe } from "./safe";
import type { Prettify } from "./utils";

export type User = {
  keys: Record<KeyHash, Key>;
  accounts: Record<Address, AccountInfo>;
};

export type Username = string;

export type AccountTypes = (typeof AccountType)[keyof typeof AccountType];

export const AccountType = {
  Spot: "spot",
  Margin: "margin",
  Safe: "safe",
} as const;

export type AccountSingleConfig = { owner: Username };
export type AccountMultiConfig = Safe;

export type AccountConfig = {
  readonly [AccountType.Spot]: AccountSingleConfig;
  readonly [AccountType.Margin]: AccountSingleConfig;
  readonly [AccountType.Safe]: AccountMultiConfig;
};

export type AccountIndex = number;

export type AccountInfo = {
  readonly index: AccountIndex;
  readonly params: AccountConfig;
};

export type Account = Prettify<
  {
    readonly username: Username;
    readonly address: Address;
    readonly type: AccountTypes;
  } & AccountInfo
>;
