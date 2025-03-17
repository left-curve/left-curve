import type { Address, Prettify } from "@left-curve/sdk/types";
import type { Key, KeyHash } from "./key.js";
import type { Safe } from "./safe.js";

export type User = {
  keys: Record<KeyHash, Key>;
  accounts: Record<Address, AccountInfo>;
};

export type Username = string;

export type AccountTypes = (typeof AccountType)[keyof typeof AccountType];

export const AccountType = {
  Spot: "spot",
  Margin: "margin",
  Multi: "multi",
} as const;

export type AccountSingleConfig = { owner: Username };
export type AccountMultiConfig = Safe;

export type AccountConfigs = {
  [AccountType.Spot]: AccountSingleConfig;
  [AccountType.Margin]: AccountSingleConfig;
  [AccountType.Multi]: AccountMultiConfig;
};

export type AccountConfig =
  | { readonly [AccountType.Spot]: AccountSingleConfig }
  | { readonly [AccountType.Margin]: AccountSingleConfig }
  | { readonly [AccountType.Multi]: AccountMultiConfig };

export type AccountIndex = number;

export type AccountInfo<accountType extends AccountTypes = AccountTypes> = {
  readonly index: AccountIndex;
  readonly params: AccountParams<accountType>;
};

export type Account<accountType extends AccountTypes = AccountTypes> = Prettify<
  {
    readonly username: Username;
    readonly address: Address;
    readonly type: accountType;
  } & AccountInfo<accountType>
>;

export type AccountParams<K extends AccountTypes | unknown = unknown> =
  K extends keyof AccountConfigs ? { readonly [P in K]: AccountConfigs[K] } : AccountConfig;
