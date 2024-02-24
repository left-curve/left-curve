import type { Coin, Config } from ".";

export type Tx = {
  sender: string;
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
  to: string;
  coins: Coin[];
}

export type MsgStoreCode = {
  wasmByteCode: string;
};

export type MsgInstantiate = {
  codeHash: string;
  msg: string;
  salt: string;
  funds: Coin[];
  admin?: string;
};

export type MsgExecute = {
  contract: string;
  msg: string;
  funds: Coin[];
};

export type MsgMigrate = {
  contract: string;
  newCodeHash: string;
  msg: string;
};
