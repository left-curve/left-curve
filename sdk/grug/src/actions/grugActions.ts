import type {
  AppConfigResponse,
  Chain,
  ChainStatusResponse,
  Client,
  JsonValue,
  Signer,
  Transport,
} from "../types/index.js";

import { type GetBalanceParameters, type GetBalanceReturnType, getBalance } from "./getBalance.js";

import {
  type GetBalancesParameters,
  type GetBalancesReturnType,
  getBalances,
} from "./getBalances.js";

import { type GetSupplyParameters, type GetSupplyReturnType, getSupply } from "./getSupply.js";

import {
  type GetSuppliesParameters,
  type GetSuppliesReturnType,
  getSupplies,
} from "./getSupplies.js";

import { type GetCodeParameters, type GetCodeReturnType, getCode } from "./getCode.js";

import { type GetCodesParameters, type GetCodesReturnType, getCodes } from "./getCodes.js";

import { type QueryStatusReturnType, queryStatus } from "./queryStatus.js";

import { type QueryAppParameters, type QueryAppReturnType, queryApp } from "./queryApp.js";

import {
  type QueryWasmRawParameters,
  type QueryWasmRawReturnType,
  queryWasmRaw,
} from "./queryWasmRaw.js";

import {
  type QueryWasmSmartParameters,
  type QueryWasmSmartReturnType,
  queryWasmSmart,
} from "./queryWasmSmart.js";

import { type SimulateParameters, type SimulateReturnType, simulate } from "./simulate.js";

import {
  type GetAppConfigParameters,
  type GetAppConfigReturnType,
  getAppConfig,
} from "./getAppConfig.js";

import {
  type GetContractInfoParameters,
  type GetContractInfoReturnType,
  getContractInfo,
} from "./getContractInfo.js";

import {
  type GetContractsInfoParameters,
  type GetContractsInfoReturnType,
  getContractsInfo,
} from "./getContractsInfo.js";

import { type QueryAbciParameters, type QueryAbciReturnType, queryAbci } from "./queryAbci.js";

import { type QueryTxParameters, type QueryTxReturnType, queryTx } from "./queryTx.js";

export type GrugActions<
  _transport extends Transport = Transport,
  _chain extends Chain | undefined = Chain | undefined,
  _signer extends Signer | undefined = undefined,
> = {
  getBalance: (args: GetBalanceParameters) => GetBalanceReturnType;
  getBalances: (args: GetBalancesParameters) => GetBalancesReturnType;
  getSupply: (args: GetSupplyParameters) => GetSupplyReturnType;
  getSupplies: (args?: GetSuppliesParameters) => GetSuppliesReturnType;
  getCode: (args: GetCodeParameters) => GetCodeReturnType;
  getCodes: (args?: GetCodesParameters) => GetCodesReturnType;
  getAppConfig: <value extends AppConfigResponse>(
    args?: GetAppConfigParameters,
  ) => GetAppConfigReturnType<value>;
  getContractInfo: (args: GetContractInfoParameters) => GetContractInfoReturnType;
  getContractsInfo: (args?: GetContractsInfoParameters) => GetContractsInfoReturnType;
  queryApp: (args: QueryAppParameters) => QueryAppReturnType;
  queryAbci: (args: QueryAbciParameters) => QueryAbciReturnType;
  queryTx: (args: QueryTxParameters) => QueryTxReturnType;
  queryStatus: <statusInfo extends ChainStatusResponse>() => QueryStatusReturnType<statusInfo>;
  queryWasmRaw: (args: QueryWasmRawParameters) => QueryWasmRawReturnType;
  queryWasmSmart: <value extends JsonValue>(
    args: QueryWasmSmartParameters,
  ) => QueryWasmSmartReturnType<value>;
  simulate: (args: SimulateParameters) => SimulateReturnType;
};

export function grugActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = undefined,
>(client: Client<transport, chain, signer>): GrugActions<transport, chain, signer> {
  return {
    getAppConfig: (args) => getAppConfig(client, args),
    getBalance: (args) => getBalance(client, args),
    getBalances: (args) => getBalances(client, args),
    getSupply: (args) => getSupply(client, args),
    getSupplies: (args) => getSupplies(client, args),
    getCode: (args) => getCode(client, args),
    getCodes: (args) => getCodes(client, args),
    queryStatus: () => queryStatus(client),
    getContractInfo: (args) => getContractInfo(client, args),
    getContractsInfo: (args) => getContractsInfo(client, args),
    queryApp: (args) => queryApp(client, args),
    queryTx: (args) => queryTx(client, args),
    queryAbci: (args) => queryAbci(client, args),
    queryWasmRaw: (args) => queryWasmRaw(client, args),
    queryWasmSmart: (args) => queryWasmSmart(client, args),
    simulate: (args) => simulate(client, args),
  };
}
