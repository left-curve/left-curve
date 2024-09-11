import type { Address } from "./address";

export type Username = string;

export type AccountTypes = (typeof AccountType)[keyof typeof AccountType];

export const AccountType = {
  Spot: "spot",
  Margin: "margin",
} as const;

export type AccountIndex = number;

export type Account = {
  username: Username;
  address: Address;
  type: AccountTypes;
};

export type AccountStateResponse = {
  sequence: number;
};
