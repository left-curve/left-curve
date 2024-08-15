import type { Coin } from "./coins";
import type { Base64, Hex, Json } from "./common";

export type Credential = { secp256k1: string } | { ed25519: string } | { passkey: unknown };

export type Metadata = {
  username: string;
  keyId: string;
  sequence: number;
};

export type Tx = {
  sender: string;
  msgs: Message[];
  gasLimit: number;
  credential: Credential;
  data: Metadata;
};

export type UnsignedTx = Pick<Tx, "sender" | "msgs" | "gasLimit"> & {
  credential: null;
  data: Json;
};

// biome-ignore format: biome's style of formatting union types is ugly
export type Message = {
	configure: MsgUpdateConfig;
  } | {
	transfer: MsgTransfer;
  } | {
	upload: MsgStoreCode;
  } | {
	instantiate: MsgInstantiate;
  } | {
	execute: MsgExecute;
  } | {
	migrate: MsgMigrate;
  };

export type MsgUpdateConfig = {
  newCfg: {
    owner?: string;
    bank: string;
  };
};

export type MsgTransfer = {
  to: string;
  coins: Coin;
};

export type MsgStoreCode = {
  code: Base64;
};

export type MsgInstantiate = {
  codeHash: Hex;
  msg: Json;
  salt: Base64;
  funds: Coin;
  admin?: string;
};

export type MsgExecute = {
  contract: string;
  msg: Json;
  funds: Coin;
};

export type MsgMigrate = {
  contract: string;
  newCodeHash: Hex;
  msg: Json;
};

export enum AdminOptionKind {
  SetToSelf = 0,
  SetToNone = 1,
}

export type AdminOption =
  | string
  | AdminOptionKind.SetToSelf
  | AdminOptionKind.SetToNone
  | undefined;
