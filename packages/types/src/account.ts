import type { Address } from "./address";
import type { Prettify } from "./utils";

export type Username = string;

export type AccountType = (typeof AccountTypes)[keyof typeof AccountTypes];

export const AccountTypes = {
  Spot: "spot",
  Margin: "margin",
} as const;

export type AccountIndex = number;

export type AccountId = `${Username}/account/${AccountIndex}`;

export type AccountInfo = {
  type: AccountType;
  address: Address;
};

export type Account = Prettify<
  {
    id: AccountId;
    index: AccountIndex;
    username: Username;
  } & AccountInfo
>;

export type AccountStateResponse = {
  sequence: number;
};
