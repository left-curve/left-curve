import type { Addr, BlockInfo, Coin, Config, Hash } from ".";

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
  address: Addr;
  denom: string;
};

export type QueryBalancesRequest = {
  address: Addr;
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
  hash: Hash;
};

export type QueryCodesRequest = {
  startAfter?: Hash;
  limit?: number;
};

export type QueryAccountRequest = {
  address: Addr;
};

export type QueryAccountsRequest = {
  startAfter?: Addr;
  limit?: number;
};

export type QueryWasmRawRequest = {
  contract: Addr;
  key: string;
};

export type QueryWasmSmartRequest = {
  contract: Addr;
  msg: string;
};

export type QueryResponse = {
  info?: InfoResponse,
  balance?: Coin,
  balances?: Coin[],
  supply?: Coin,
  supplies?: Coin[],
  code?: string,
  codes?: Hash[],
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
  address: Addr;
  codeHash: Hash;
  admin?: string;
};

export type WasmRawResponse = {
  contract: Addr;
  key: string;
  value?: string;
};

export type WasmSmartResponse = {
  contract: Addr;
  data: string;
};
