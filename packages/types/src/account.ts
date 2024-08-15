import type { Hex } from "./common";
import type { Credential, Message, Metadata } from "./tx";

export type Address = `0x${string}`;

export type Account = {
  username: string;
  computeAddress: (
    username: string,
    factoryAddr: string,
    accountTypeCodeHash: string,
  ) => Promise<Address>;
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
