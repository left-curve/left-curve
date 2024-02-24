import type { BlockInfo, Coin, Config } from ".";

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
