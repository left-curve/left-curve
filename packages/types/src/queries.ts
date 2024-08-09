import type { Coin } from "./coins";

export type BlockInfo = {
  height: string;
  timestamp: string;
  hash: string;
};

export type ChainConfig = {
  owner?: string;
  bank: string;
};

export type QueryRequest = {
  info: QueryInfoRequest;
} | {
  balance: QueryBalanceRequest;
} | {
  balances: QueryBalancesRequest;
} | {
  supply: QuerySupplyRequest;
} | {
  supplies: QuerySuppliesReuest;
} | {
  code: QueryCodeRequest;
} | {
  codes: QueryCodesRequest;
} | {
  account: QueryAccountRequest;
} | {
  accounts: QueryAccountsRequest;
} | {
  wasmRaw: QueryWasmRawRequest;
} | {
  wasmSmart: QueryWasmSmartRequest;
};

// The info request is just an empty object (`{}`), but we can't define it that
// way, because of: https://typescript-eslint.io/rules/ban-types/#:~:text=Avoid%20the%20Object%20and%20%7B%7D%20types
export type QueryInfoRequest = Record<string, never>;

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

// biome-ignore format: biome's style of formatting union types is ugly
export type QueryResponse = {
  info: InfoResponse;
} | {
  balance: Coin;
} | {
  balances: Coin[];
} | {
  supply: Coin;
} | {
  supplies: Coin[];
} | {
  code: string;
} | {
  codes: string[];
} | {
  account: AccountResponse;
} | {
  accounts: AccountResponse[];
} | {
  wasmRaw: WasmRawResponse;
} | {
  wasmSmart: WasmSmartResponse;
};

export type InfoResponse = {
  chainId: string;
  config: ChainConfig;
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
