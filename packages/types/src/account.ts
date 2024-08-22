import type { Address } from "./address";
import type { Hex } from "./common";
import type { Credential, Metadata } from "./credential";
import type { Message } from "./tx";

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

export type Account = {
  username: Username;
  getKeyId: () => Promise<Hex>;
  signTx: (
    msgs: Message[],
    chainId: string,
    sequence: number,
  ) => Promise<{ credential: Credential; data: Metadata }>;
};

export type AccountStateResponse = {
  sequence: number;
};
