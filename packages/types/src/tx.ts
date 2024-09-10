import type { Address } from "./address";
import type { Coins } from "./coin";
import type { Duration } from "./common";
import type { Credential, Metadata } from "./credential";
import type { Base64, Hex, Json } from "./encoding";

export type TxParameters = {
  funds?: Coins;
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
  funds?: Coins;
  admin?: string;
};

export type MsgExecute = {
  contract: Address;
  msg: Json;
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
  cronjobs?: Record<Address, Duration>;
  permissions?: {
    upload: Permission;
    instantiate: Permission;
  };
};

/**
 * Only the owner can perform the action. Note, the owner is always able to
 * upload code or instantiate contracts.
 */
export type NobodyPermission = "nobody";
/**
 * Any account is allowed to perform the action
 */
export type EverybodyPermission = "everybody";
/**
 * Some whitelisted accounts or the owner can perform the action.
 */
export type SomebodiesPermission = { somebodies: Address[] };

/**
 * Permissions for uploading code or instantiating contracts.
 */
export type Permission = NobodyPermission | EverybodyPermission | SomebodiesPermission;
