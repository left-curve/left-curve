import type { Address } from "./address.js";
import type { ChainConfig } from "./app.js";
import type { Coins, Funds } from "./coin.js";
import type { Base64, Hex, Json } from "./encoding.js";

export type TxOutcome = {
  gasLimit: number;
  gasUsed: number;
};

export type TxParameters = {
  gasLimit?: number;
};

export type Tx<Credential = Json, Metadata = Json> = {
  sender: Address;
  msgs: Message[];
  gasLimit: number;
  credential: Credential;
  data: Metadata;
};

export type UnsignedTx = Pick<Tx, "sender" | "msgs">;

export type Message =
  | { configure: MsgConfigure }
  | { transfer: MsgTransfer }
  | { upload: MsgStoreCode }
  | { instantiate: MsgInstantiate }
  | { execute: MsgExecute }
  | { migrate: MsgMigrate };

export type MsgConfigure<AppConfig = Json> = {
  newCfg: Partial<ChainConfig>;
  newAppCfg: Partial<AppConfig>;
};

export type MsgTransfer = Record<Address, Coins>;

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
