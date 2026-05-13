import type { AppConfig, Client, JsonValue, Signer } from "@left-curve/types";

import {
  type BroadcastTxSyncParameters,
  type BroadcastTxSyncReturnType,
  broadcastTxSync,
} from "./mutations/broadcastTxSync.js";
import { type ExecuteParameters, type ExecuteReturnType, execute } from "./mutations/execute.js";
import {
  type InstantiateParameters,
  type InstantiateReturnType,
  instantiate,
} from "./mutations/instantiate.js";
import { type MigrateParameters, type MigrateReturnType, migrate } from "./mutations/migrate.js";
import {
  type SignAndBroadcastTxParameters,
  type SignAndBroadcastTxReturnType,
  signAndBroadcastTx,
} from "./mutations/signAndBroadcastTx.js";
import {
  type StoreCodeParameters,
  type StoreCodeReturnType,
  storeCode,
} from "./mutations/storeCode.js";
import {
  type StoreCodeAndInstantiateParameters,
  type StoreCodeAndInstantiateReturnType,
  storeCodeAndInstantiate,
} from "./mutations/storeCodeAndInstantiate.js";
import {
  type TransferParameters,
  type TransferReturnType,
  transfer,
} from "./mutations/transfer.js";

import { getAppConfig } from "./queries/getAppConfig.js";
import { type QueryAppParameters, type QueryAppReturnType, queryApp } from "./queries/queryApp.js";
import { type QueryStatusReturnType, queryStatus } from "./queries/queryStatus.js";
import { type QueryTxParameters, type QueryTxReturnType, queryTx } from "./queries/queryTx.js";
import { type SimulateParameters, type SimulateReturnType, simulate } from "./queries/simulate.js";

import {
  type GetBalanceParameters,
  type GetBalanceReturnType,
  getBalance,
} from "./queries/getBalance.js";
import {
  type GetBalancesParameters,
  type GetBalancesReturnType,
  getBalances,
} from "./queries/getBalances.js";
import {
  type GetSupplyParameters,
  type GetSupplyReturnType,
  getSupply,
} from "./queries/getSupply.js";
import {
  type GetSuppliesParameters,
  type GetSuppliesReturnType,
  getSupplies,
} from "./queries/getSupplies.js";
import { type GetCodeParameters, type GetCodeReturnType, getCode } from "./queries/getCode.js";
import { type GetCodesParameters, type GetCodesReturnType, getCodes } from "./queries/getCodes.js";
import {
  type GetContractInfoParameters,
  type GetContractInfoReturnType,
  getContractInfo,
} from "./queries/getContractInfo.js";
import {
  type GetContractsInfoParameters,
  type GetContractsInfoReturnType,
  getContractsInfo,
} from "./queries/getContractsInfo.js";
import {
  type QueryWasmRawParameters,
  type QueryWasmRawReturnType,
  queryWasmRaw,
} from "./queries/queryWasmRaw.js";
import {
  type QueryWasmSmartParameters,
  type QueryWasmSmartReturnType,
  queryWasmSmart,
} from "./queries/queryWasmSmart.js";

export type AppQueryActions = {
  getAppConfig: () => Promise<AppConfig>;
  getBalance: (args: GetBalanceParameters) => GetBalanceReturnType;
  getBalances: (args: GetBalancesParameters) => GetBalancesReturnType;
  getSupply: (args: GetSupplyParameters) => GetSupplyReturnType;
  getSupplies: (args?: GetSuppliesParameters) => GetSuppliesReturnType;
  getCode: (args: GetCodeParameters) => GetCodeReturnType;
  getCodes: (args?: GetCodesParameters) => GetCodesReturnType;
  getContractInfo: (args: GetContractInfoParameters) => GetContractInfoReturnType;
  getContractsInfo: (args?: GetContractsInfoParameters) => GetContractsInfoReturnType;
  queryTx(args: QueryTxParameters): QueryTxReturnType;
  queryApp(args: QueryAppParameters): QueryAppReturnType;
  queryStatus(): QueryStatusReturnType;
  queryWasmRaw: (args: QueryWasmRawParameters) => QueryWasmRawReturnType;
  queryWasmSmart: <value extends JsonValue>(
    args: QueryWasmSmartParameters,
  ) => QueryWasmSmartReturnType<value>;
  simulate(args: SimulateParameters): SimulateReturnType;
};

export type AppMutationActions = {
  broadcastTxSync(args: BroadcastTxSyncParameters): BroadcastTxSyncReturnType;
  execute(args: ExecuteParameters): ExecuteReturnType;
  instantiate(args: InstantiateParameters): InstantiateReturnType;
  migrate(args: MigrateParameters): MigrateReturnType;
  signAndBroadcastTx(args: SignAndBroadcastTxParameters): SignAndBroadcastTxReturnType;
  storeCode(args: StoreCodeParameters): StoreCodeReturnType;
  storeCodeAndInstantiate(
    args: StoreCodeAndInstantiateParameters,
  ): StoreCodeAndInstantiateReturnType;
  transfer(args: TransferParameters): TransferReturnType;
};

export function appQueryActions(client: Client): AppQueryActions {
  return {
    getAppConfig: () => getAppConfig(client),
    getBalance: (args) => getBalance(client, args),
    getBalances: (args) => getBalances(client, args),
    getSupply: (args) => getSupply(client, args),
    getSupplies: (args) => getSupplies(client, args),
    getCode: (args) => getCode(client, args),
    getCodes: (args) => getCodes(client, args),
    getContractInfo: (args) => getContractInfo(client, args),
    getContractsInfo: (args) => getContractsInfo(client, args),
    queryTx: (args) => queryTx(client, args),
    queryApp: (args) => queryApp(client, args),
    queryStatus: () => queryStatus(client),
    queryWasmRaw: (args) => queryWasmRaw(client, args),
    queryWasmSmart: (args) => queryWasmSmart(client, args),
    simulate: (args) => simulate(client, args),
  };
}

export function appMutationActions(client: Client<Signer>): AppMutationActions {
  return {
    broadcastTxSync: (args) => broadcastTxSync(client, args),
    execute: (args) => execute(client, args),
    instantiate: (args) => instantiate(client, args),
    migrate: (args) => migrate(client, args),
    signAndBroadcastTx: (args) => signAndBroadcastTx(client, args),
    storeCode: (args) => storeCode(client, args),
    storeCodeAndInstantiate: (args) => storeCodeAndInstantiate(client, args),
    transfer: (args) => transfer(client, args),
  };
}
