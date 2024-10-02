import type { Address } from "./address";
import type { Coin, Coins } from "./coin";
import type { Metadata } from "./credential";
import type { Hex, Json } from "./encoding";
import type { Message } from "./tx";

export type BlockInfo = {
  height: string;
  timestamp: string;
  hash: string;
};

export type ContractInfo = {
  codeHash: Hex;
  admin?: Address;
};

export type ChainConfig = {
  owner?: string;
  bank: string;
};

export type SimulateRequest = {
  sender: string;
  msgs: Message[];
  data: Metadata | null;
};

export type SimulateResponse = {
  gasLimit: number;
  gasUsed: number;
};

export type QueryRequest =
  | { info: QueryInfoRequest }
  | { appConfig: QueryAppConfigRequest }
  | { appConfigs: QueryAppConfigsRequest }
  | { balance: QueryBalanceRequest }
  | { balances: QueryBalancesRequest }
  | { supply: QuerySupplyRequest }
  | { supplies: QuerySuppliesReuest }
  | { code: QueryCodeRequest }
  | { codes: QueryCodesRequest }
  | { contract: QueryContractRequest }
  | { contracts: QueryContractsRequest }
  | { wasmRaw: QueryWasmRawRequest }
  | { wasmSmart: QueryWasmSmartRequest };

// The info request is just an empty object (`{}`), but we can't define it that
// way, because of: https://typescript-eslint.io/rules/ban-types/#:~:text=Avoid%20the%20Object%20and%20%7B%7D%20types
export type QueryInfoRequest = Record<string, never>;

export type QueryAppConfigRequest = {
  key: string;
};

export type QueryAppConfigsRequest = {
  startAfter?: string;
  limit?: number;
};

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

export type QueryWasmRawRequest = {
  contract: string;
  key: string;
};

export type QueryWasmSmartRequest = {
  contract: string;
  msg: Json;
};

export type QueryContractsRequest = {
  startAfter?: Address;
  limit?: number;
};

export type QueryContractRequest = {
  address: Address;
};

export type QueryResponse =
  | { info: InfoResponse }
  | { balance: Coin }
  | { appConfig: AppConfigResponse }
  | { appConfigs: AppConfigsResponse }
  | { balances: Coins }
  | { supply: Coin }
  | { supplies: Coins }
  | { code: string }
  | { codes: string[] }
  | { contract: ContractResponse }
  | { contracts: ContractsResponse }
  | { wasmRaw: WasmRawResponse }
  | { wasmSmart: WasmSmartResponse };

export type InfoResponse = {
  chainId: string;
  config: ChainConfig;
  lastFinalizedBlock: BlockInfo;
};

export type AppConfigResponse = Json;

export type AppConfigsResponse = Record<string, Json>;

export type AccountResponse = {
  address: Address;
  codeHash: string;
  admin?: string;
};

export type WasmRawResponse = {
  contract: Address;
  key: string;
  value?: string;
};

export type WasmSmartResponse = {
  contract: Address;
  data: string;
};

export type ContractResponse = ContractInfo;

export type ContractsResponse = Record<Address, ContractInfo>;
