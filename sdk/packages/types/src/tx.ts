import type { Address } from "./address";
import type { Coins, Funds } from "./coin";
import type { Duration, Permission } from "./common";
import type { Credential, Metadata } from "./credential";
import type { Base64, Hex, Json } from "./encoding";

export type TxParameters = {
  funds?: Funds;
  gasLimit?: number;
};

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
  appUpdates: Record<string, Json>;
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
  funds?: Funds;
  admin?: string;
};

export type MsgExecute = {
  contract: Address;
  msg: Json;
  funds?: Funds;
};

export type MsgMigrate = {
  contract: Address;
  newCodeHash: Hex;
  msg: Json;
};

export type ConfigUpdate = {
  owner?: Address;
  bank?: Address;
  taxman?: Address;
  cronjobs?: Record<Address, Duration>;
  permissions?: {
    upload: Permission;
    instantiate: Permission;
  };
};
