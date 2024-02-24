import type { Addr, Coin, Config, Hash } from ".";

export type Tx = {
  sender: Addr;
  msgs: Message[];
  credential: string;
};

export type Message = {
  updateConfig?: MsgUpdateConfig;
  transfer?: MsgTransfer;
  storeCode?: MsgStoreCode;
  instantiate?: MsgInstantiate;
  execute?: MsgExecute;
  migrate?: MsgMigrate;
};

export type MsgUpdateConfig = {
  newCfg: Config;
};

export type MsgTransfer = {
  to: Addr;
  coins: Coin[];
}

export type MsgStoreCode = {
  wasmByteCode: string;
};

export type MsgInstantiate = {
  codeHash: Hash;
  msg: string;
  salt: string;
  funds: Coin[];
  admin?: Addr;
};

export type MsgExecute = {
  contract: Addr;
  msg: string;
  funds: Coin[];
};

export type MsgMigrate = {
  contract: Addr;
  newCodeHash: Hash;
  msg: string;
};
