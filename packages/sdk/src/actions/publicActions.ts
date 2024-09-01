import type { Chain, Client, Signer, Transport } from "@leftcurve/types";

import {
  type GetBalanceParameters,
  type GetBalanceReturnType,
  getBalance,
} from "./public/getBalance";

import {
  type GetBalancesParameters,
  type GetBalancesReturnType,
  getBalances,
} from "./public/getBalances";

import { type GetSupplyParameters, type GetSupplyReturnType, getSupply } from "./public/getSupply";

import {
  type GetSuppliesParameters,
  type GetSuppliesReturnType,
  getSupplies,
} from "./public/getSupplies";

import { type GetCodeParameters, type GetCodeReturnType, getCode } from "./public/getCode";

import { type GetCodesParameters, type GetCodesReturnType, getCodes } from "./public/getCodes";

import {
  type GetChainInfoParameters,
  type GetChainInfoReturnType,
  getChainInfo,
} from "./public/getChainInfo";

import { type QueryAppParameters, type QueryAppReturnType, queryApp } from "./public/queryApp";

import {
  type QueryWasmRawParameters,
  type QueryWasmRawReturnType,
  queryWasmRaw,
} from "./public/queryWasmRaw";

import {
  type QueryWasmSmartParameters,
  type QueryWasmSmartReturnType,
  queryWasmSmart,
} from "./public/queryWasmSmart";

import {
  type CreateAccountParameters,
  type CreateAccountReturnType,
  createAccount,
} from "./public/createAccount";

import { type SimulateParameters, type SimulateReturnType, simulate } from "./public/simulate";

import {
  type ComputeAddressParameters,
  type ComputeAddressReturnType,
  computeAddress,
} from "./public/computeAddress";

import {
  type GetAppConfigParameters,
  type GetAppConfigReturnType,
  getAppConfig,
} from "./public/getAppConfig";

import {
  type GetAppConfigsParameters,
  type GetAppConfigsReturnType,
  getAppConfigs,
} from "./public/getAppConfigs";

import {
  type GetAccountTypeCodeHashParameters,
  type GetAccountTypeCodeHashReturnType,
  getAccountTypeCodeHash,
} from "./public/getAccountTypeCodeHash";

import {
  type GetAccountTypeCodeHashesParameters,
  type GetAccountTypeCodeHashesReturnType,
  getAccountTypeCodeHashes,
} from "./public/getAccountTypeCodeHashes";

import {
  type GetAccountsByKeyHashParameters,
  type GetAccountsByKeyHashReturnType,
  getAccountsByKeyHash,
} from "./public/getAccountsByKeyHash";

import {
  type GetKeysByUsernameParameters,
  type GetKeysByUsernameReturnType,
  getKeysByUsername,
} from "./public/getKeysByUsername";

import {
  type GetPublicKeyFromKeyIdParameters,
  type GetPublicKeyFromKeyIdReturnType,
  getPublicKeyFromKeyId,
} from "./public/getPublicKeyFromKeyId";

import {
  type GetAccountIdByAddressParameters,
  type GetAccountIdByAddressReturnType,
  getAccountIdByAddress,
} from "./public/getAccountIdByAddress";

import {
  type GetAccountInfoByAccountIdParameters,
  type GetAccountInfoByAccountIdReturnType,
  getAccountInfoByAccountId,
} from "./public/getAccountInfoByAccountId";

import {
  type GetAccountsByUsernameParameters,
  type GetAccountsByUsernameReturnType,
  getAccountsByUsername,
} from "./public/getAccountsByUsername";

import {
  type GetContractInfoParameters,
  type GetContractInfoReturnType,
  getContractInfo,
} from "./public/getContractInfo";

import {
  type GetContractsInfoParameters,
  type GetContractsInfoReturnType,
  getContractsInfo,
} from "./public/getContractsInfo";

export type PublicActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = undefined,
> = {
  getBalance: (args: GetBalanceParameters) => GetBalanceReturnType;
  getBalances: (args: GetBalancesParameters) => GetBalancesReturnType;
  getSupply: (args: GetSupplyParameters) => GetSupplyReturnType;
  getSupplies: (args?: GetSuppliesParameters) => GetSuppliesReturnType;
  getCode: (args: GetCodeParameters) => GetCodeReturnType;
  getCodes: (args?: GetCodesParameters) => GetCodesReturnType;
  getChainInfo: (args?: GetChainInfoParameters) => GetChainInfoReturnType;
  getAppConfig: <value extends any | undefined>(
    args: GetAppConfigParameters,
  ) => GetAppConfigReturnType<value>;
  getAppConfigs: (args?: GetAppConfigsParameters) => GetAppConfigsReturnType;
  getAccountTypeCodeHash: (
    args: GetAccountTypeCodeHashParameters,
  ) => GetAccountTypeCodeHashReturnType;
  getAccountTypeCodeHashes: (
    args?: GetAccountTypeCodeHashesParameters,
  ) => GetAccountTypeCodeHashesReturnType;
  getAccountsByKeyHash: (args: GetAccountsByKeyHashParameters) => GetAccountsByKeyHashReturnType;
  getPublicKeyFromKeyId: (args: GetPublicKeyFromKeyIdParameters) => GetPublicKeyFromKeyIdReturnType;
  getKeysByUsername: (args: GetKeysByUsernameParameters) => GetKeysByUsernameReturnType;
  getAccountInfoByAccountId: (
    args: GetAccountInfoByAccountIdParameters,
  ) => GetAccountInfoByAccountIdReturnType;
  getAccountsByUsername: (args: GetAccountsByUsernameParameters) => GetAccountsByUsernameReturnType;
  getAccountIdByAddress: (args: GetAccountIdByAddressParameters) => GetAccountIdByAddressReturnType;
  getContractInfo: (args: GetContractInfoParameters) => GetContractInfoReturnType;
  getContractsInfo: (args?: GetContractsInfoParameters) => GetContractsInfoReturnType;
  queryApp: (args: QueryAppParameters) => QueryAppReturnType;
  queryWasmRaw: <value extends any | undefined>(
    args: QueryWasmRawParameters,
  ) => QueryWasmRawReturnType<value>;
  queryWasmSmart: <value extends any | undefined>(
    args: QueryWasmSmartParameters,
  ) => QueryWasmSmartReturnType<value>;
  createAccount: (args: CreateAccountParameters) => CreateAccountReturnType;
  simulate: (args: SimulateParameters) => SimulateReturnType;
  computeAddress: (args: ComputeAddressParameters) => ComputeAddressReturnType;
};

export function publicActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = undefined,
>(client: Client<transport, chain, signer>): PublicActions<transport, chain, signer> {
  return {
    getAppConfig: (args) => getAppConfig(client, args),
    getAppConfigs: (args) => getAppConfigs(client, args),
    getBalance: (args) => getBalance(client, args),
    getBalances: (args) => getBalances(client, args),
    getSupply: (args) => getSupply(client, args),
    getSupplies: (args) => getSupplies(client, args),
    getCode: (args) => getCode(client, args),
    getCodes: (args) => getCodes(client, args),
    getChainInfo: (args) => getChainInfo(client, args),
    getAccountTypeCodeHash: (args) => getAccountTypeCodeHash(client, args),
    getAccountTypeCodeHashes: (args) => getAccountTypeCodeHashes(client, args),
    getAccountsByKeyHash: (args) => getAccountsByKeyHash(client, args),
    getPublicKeyFromKeyId: (args) => getPublicKeyFromKeyId(client, args),
    getKeysByUsername: (args) => getKeysByUsername(client, args),
    getAccountInfoByAccountId: (args) => getAccountInfoByAccountId(client, args),
    getAccountsByUsername: (args) => getAccountsByUsername(client, args),
    getAccountIdByAddress: (args) => getAccountIdByAddress(client, args),
    getContractInfo: (args) => getContractInfo(client, args),
    getContractsInfo: (args) => getContractsInfo(client, args),
    queryApp: (args) => queryApp(client, args),
    queryWasmRaw: (args) => queryWasmRaw(client, args),
    queryWasmSmart: (args) => queryWasmSmart(client, args),
    createAccount: (args) => createAccount(client, args),
    simulate: (args) => simulate(client, args),
    computeAddress: (args) => computeAddress(args),
  };
}
