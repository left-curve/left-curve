import type { Address } from "./address";

export type Username = string;

export type AccountType = (typeof AccountType)[keyof typeof AccountType];

export const AccountType = {
  Spot: "spot",
  Margin: "margin",
} as const;

export type AccountIndex = number;

export type Account = {
  username: Username;
  address: Address;
  type: AccountType;
};

export type AccountStateResponse = {
  sequence: number;
};
