import type { Addr, Binary, Coin, Config, Hash } from ".";

export type Tx = {
  sender: Addr;
  msgs: Message[];
  credential: Binary;
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
};

export type MsgStoreCode = {
  wasmByteCode: Binary;
};

export type MsgInstantiate = {
  codeHash: Hash;
  msg: Binary;
  salt: Binary;
  funds: Coin[];
  admin?: Addr;
};

export type MsgExecute = {
  contract: Addr;
  msg: Binary;
  funds: Coin[];
};

export type MsgMigrate = {
  contract: Addr;
  newCodeHash: Hash;
  msg: Binary;
};
