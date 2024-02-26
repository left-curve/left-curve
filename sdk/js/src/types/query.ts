import type { Addr, Binary, BlockInfo, Coin, Config, Hash } from ".";

export type QueryRequest = {
  info?: QueryInfoRequest;
  balance?: QueryBalanceRequest;
  balances?: QueryBalancesRequest;
  supply?: QuerySupplyRequest;
  supplies?: QuerySuppliesReuest;
  code?: QueryCodeRequest;
  codes?: QueryCodesRequest;
  account?: QueryAccountRequest;
  accounts?: QueryAccountsRequest;
  wasmRaw?: QueryWasmRawRequest;
  wasmSmart?: QueryWasmSmartRequest;
};

// https://typescript-eslint.io/rules/ban-types/#:~:text=Avoid%20the%20Object%20and%20%7B%7D%20types
export type QueryInfoRequest = Record<string, never>;

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
  key: Binary;
};

export type QueryWasmSmartRequest = {
  contract: Addr;
  msg: Binary;
};

export type QueryResponse = {
  info?: InfoResponse;
  balance?: Coin;
  balances?: Coin[];
  supply?: Coin;
  supplies?: Coin[];
  code?: Binary;
  codes?: Hash[];
  account?: AccountResponse;
  accounts?: AccountResponse[];
  wasmRaw?: WasmRawResponse;
  wasmSmart?: WasmSmartResponse;
};

export type InfoResponse = {
  chainId: string;
  config: Config;
  lastFinalizedBlock: BlockInfo;
};

export type AccountResponse = {
  address: Addr;
  codeHash: Hash;
  admin?: Addr;
};

export type WasmRawResponse = {
  contract: Addr;
  key: Binary;
  value?: Binary;
};

export type WasmSmartResponse = {
  contract: Addr;
  data: Binary;
};
