import type { Address } from "./address.js";
import type { ChainConfig } from "./app.js";
import type { Coins, Funds } from "./coins.js";
import type { Base64, Hex, Json } from "./encoding.js";
import type { ExtractFromUnion, KeyOfUnion } from "./utils.js";

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
  /**  Update the chain- and app-level configurations. */
  | { configure: MsgConfigure }
  /**  Schedule a chain upgrade at a future block. */
  | { upgrade: MsgUpgrade }
  /**  Send coins to the given recipient address. */
  | { transfer: MsgTransfer }
  /**  Upload a Wasm binary code and store it in the chain's state. */
  | { upload: MsgStoreCode }
  /**  Instantiate a new contract. */
  | { instantiate: MsgInstantiate }
  /**  Execute a contract. */
  | { execute: MsgExecute }
  /**  Update the code hash associated with a contract. */
  | { migrate: MsgMigrate };

export type GetTxMesssage<K extends KeyOfUnion<Message>> = ExtractFromUnion<Message, K>;

export type MsgUpgrade = {
  height: number;
  cargoVersion: string;
  gitTag?: string;
  url?: string;
};

export type MsgConfigure<AppConfig = Json> = {
  newCfg?: ChainConfig;
  newAppCfg?: AppConfig;
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
