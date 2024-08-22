import type { Address } from "./address";
import type { Coins } from "./coin";
import type { Base64, Hex, Json } from "./common";
import type { Credential, Metadata } from "./credential";

export type Tx = {
  sender: Address;
  msgs: Message[];
  gasLimit: number;
  credential: Credential;
  data: Metadata;
};

export type UnsignedTx = Pick<Tx, "sender" | "msgs">;

export type Message =
  | { configure: MsgUpdateConfig }
  | { transfer: MsgTransfer }
  | { upload: MsgStoreCode }
  | { instantiate: MsgInstantiate }
  | { execute: MsgExecute }
  | { migrate: MsgMigrate };

export type MsgUpdateConfig = {
  updates: ConfigUpdate;
  app_updates: Record<string, unknown>;
};

export type MsgTransfer = {
  to: string;
  coins: Coins;
};

export type MsgStoreCode = {
  code: Base64;
};

export type MsgInstantiate = {
  codeHash: Hex;
  msg: Json;
  salt: Base64;
  funds?: Coins;
  admin?: string;
};

export type MsgExecute = {
  contract: Address;
  msg: any;
  funds?: Coins;
};

export type MsgMigrate = {
  contract: Address;
  newCodeHash: Hex;
  msg: Json;
};

export type ConfigUpdate = {
  owner?: Hex;
  bank?: Hex;
  taxman?: Hex;
  cronjobs?: Record<Extract<Hex, string>, number>;
  permissions?: {
    upload: unknown;
    instantiate: unknown;
  };
};
