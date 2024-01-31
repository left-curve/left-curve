// ------------------------------- query request -------------------------------

// there are probably better ways to represent the enum in typescript, but I find
// this way the easiest to work with
export type QueryRequest = {
  info?: QueryInfoRequest,
  balance?: QueryBalanceRequest,
  balances?: QueryBalancesRequest,
  supply?: QuerySupplyRequest,
  supplies?: QuerySuppliesReuest,
  code?: QueryCodeRequest,
  codes?: QueryCodesRequest,
  account?: QueryAccountRequest,
  accounts?: QueryAccountsRequest,
  wasmRaw?: QueryWasmRawRequest,
  wasmSmart?: QueryWasmSmartRequest,
};

export type QueryInfoRequest = {};

export type QueryBalanceRequest = {
  address: string;
  denom: string;
};

export type QueryBalancesRequest = {
  address: string;
  startAfter?: string;
  limit?: number;
};

export type QuerySupplyRequest = {
  denom: string;
};

export type QuerySuppliesReuest = {
  startAfter?: string;
  limit?: number;
};

export type QueryCodeRequest = {
  hash: string;
};

export type QueryCodesRequest = {
  startAfter?: string;
  limit?: number;
};

export type QueryAccountRequest = {
  address: string;
};

export type QueryAccountsRequest = {
  startAfter?: string;
  limit?: number;
};

export type QueryWasmRawRequest = {
  contract: string;
  key: string;
};

export type QueryWasmSmartRequest = {
  contract: string;
  msg: string;
};

// ------------------------------ query response -------------------------------

export type QueryResponse = {
  info?: InfoResponse,
  balance?: Coin,
  balances?: Coin[],
  supply?: Coin,
  supplies?: Coin[],
  code?: string,
  codes?: string[],
  account?: AccountResponse,
  accounts?: AccountResponse[],
  wasmRaw?: WasmRawResponse,
  wasmSmart?: WasmSmartResponse,
};

export type InfoResponse = {
  chainId: string;
  config: Config;
  lastFinalizedBlock: BlockInfo;
};

export type AccountResponse = {
  address: string;
  codeHash: string;
  admin?: string;
};

export type WasmRawResponse = {
  contract: string;
  key: string;
  value?: string;
};

export type WasmSmartResponse = {
  contract: string;
  data: string;
};

// ------------------------------------ tx -------------------------------------

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

// ---------------------------------- genesis ----------------------------------

export type GenesisState = {
  config: Config;
  msgs: Message[];
};

// -------------------------------- cw-account ---------------------------------

export type PubKey = { secp256k1: string } | { secp256r1: string };

export type AccountStateResponse = {
  pubkey: PubKey;
  sequence: number;
};

// -------------------------------- other types --------------------------------

export type Config = {
  owner?: string;
  bank: string;
};

export type BlockInfo = {
  height: number;
  timestamp: number;
};

export type Account = {
  codeHash: string;
  admin?: string;
};

export type Coin = {
  denom: string;
  amount: string;
};
