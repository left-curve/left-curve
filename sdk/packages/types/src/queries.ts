import type { Address } from "./address.js";
import type { Code } from "./code.js";
import type { Coin, Coins, Denom } from "./coin.js";
import type { Duration, Permission } from "./common.js";
import type { Metadata } from "./credential.js";
import type { Base64, Hex, Json, JsonValue } from "./encoding.js";
import type { Message } from "./tx.js";

export type BlockInfo = {
  height: string;
  timestamp: string;
  hash: string;
};

export type ContractInfo = {
  codeHash: Hex;
  label?: string;
  admin?: Address;
};

export type ChainConfigResponse = {
  owner: string;
  bank: Address;
  taxman: Address;
  cronjobs: Record<Address, Duration>;
  permissions: {
    upload: Permission;
    instantiate: Permission;
  };
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
  | { config: QueryConfigRequest }
  | { appConfig: QueryAppConfigRequest }
  | { appConfigs: QueryAppConfigsRequest }
  | { balance: QueryBalanceRequest }
  | { balances: QueryBalancesRequest }
  | { supply: QuerySupplyRequest }
  | { supplies: QuerySuppliesRequest }
  | { code: QueryCodeRequest }
  | { codes: QueryCodesRequest }
  | { contract: QueryContractRequest }
  | { contracts: QueryContractsRequest }
  | { wasmRaw: QueryWasmRawRequest }
  | { wasmSmart: QueryWasmSmartRequest }
  | { multi: QueryRequest[] };

export type QueryConfigRequest = Record<never, never>;

export type QueryAppConfigRequest = Record<never, never>;

export type QueryAppConfigsRequest = {
  startAfter?: string;
  limit?: number;
};

export type QueryBalanceRequest = {
  address: Address;
  denom: Denom;
};

export type QueryBalancesRequest = {
  address: Address;
  startAfter?: Denom;
  limit?: number;
};

export type QuerySupplyRequest = {
  denom: Denom;
};

export type QuerySuppliesRequest = {
  startAfter?: Denom;
  limit?: number;
};

export type QueryCodeRequest = {
  hash: Hex;
};

export type QueryCodesRequest = {
  startAfter?: string;
  limit?: number;
};

export type QueryWasmRawRequest = {
  contract: Address;
  key: Base64;
};

export type QueryWasmSmartRequest = {
  contract: Address;
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
  | { config: ChainConfigResponse }
  | { appConfig: AppConfigResponse }
  | { balance: Coin }
  | { balances: Coins }
  | { supply: Coin }
  | { supplies: Coins }
  | { code: CodeResponse }
  | { codes: CodesResponse }
  | { contract: ContractResponse }
  | { contracts: ContractsResponse }
  | { wasmRaw: WasmRawResponse }
  | { wasmSmart: WasmSmartResponse }
  | { multi: QueryResponse[] };

export type ChainInfoResponse = {
  chainId: string;
  config: ChainConfigResponse;
  lastFinalizedBlock: BlockInfo;
};

export type CodeResponse = Code;

export type CodesResponse = Record<Hex, Code>;

export type AppConfigResponse = Json;

export type AccountResponse = {
  address: Address;
  codeHash: string;
  admin?: string;
};

export type WasmRawResponse = Base64 | undefined;

export type WasmSmartResponse<T = JsonValue> = T;

export type ContractResponse = ContractInfo;

export type ContractsResponse = Record<Address, ContractInfo>;
