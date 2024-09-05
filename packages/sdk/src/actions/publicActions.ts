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
  type GetUsersByKeyHashReturnType,
  type GetUsersByKeyhashParameters,
  getUsersByKeyHash,
} from "./public/getUsersByKeyHash";

import {
  type GetKeysByUsernameParameters,
  type GetKeysByUsernameReturnType,
  getKeysByUsername,
} from "./public/getKeysByUsername";

import { type GetKeyParameters, type GetKeyReturnType, getKey } from "./public/getKey";

import {
  type GetUsernameByAddressParameters,
  type GetUsernameByAddressreturnType,
  getUsernameByAddress,
} from "./public/getUsernameByAddress";

import {
  type GetAccountTypeParameters,
  type GetAccountTypeReturnType,
  getAccountType,
} from "./public/getAccountType";

import {
  type GetAccountsByUsernameParameters,
  type GetAccountsByUsernameReturnType,
  getAccountsByUsername,
} from "./public/getAccountsByUsername";

import {
  type GetNextAccountIndexParameters,
  type GetNextAccountIndexReturnType,
  getNextAccountIndex,
} from "./public/getNextAccountIndex";

import {
  type GetNextAccountAddressParameters,
  type GetNextAccountAddressReturnType,
  getNextAccountAddress,
} from "./public/getNextAccountAddress";

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
  getUsersByKeyHash: (args: GetUsersByKeyhashParameters) => GetUsersByKeyHashReturnType;
  getKey: (args: GetKeyParameters) => GetKeyReturnType;
  getKeysByUsername: (args: GetKeysByUsernameParameters) => GetKeysByUsernameReturnType;
  getAccountType: (args: GetAccountTypeParameters) => GetAccountTypeReturnType;
  getAccountsByUsername: (args: GetAccountsByUsernameParameters) => GetAccountsByUsernameReturnType;
  getUsernameByAddress: (args: GetUsernameByAddressParameters) => GetUsernameByAddressreturnType;
  getNextAccountIndex: (args: GetNextAccountIndexParameters) => GetNextAccountIndexReturnType;
  getNextAccountAddress: (args: GetNextAccountAddressParameters) => GetNextAccountAddressReturnType;
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
    getUsersByKeyHash: (args) => getUsersByKeyHash(client, args),
    getKey: (args) => getKey(client, args),
    getKeysByUsername: (args) => getKeysByUsername(client, args),
    getAccountType: (args) => getAccountType(client, args),
    getAccountsByUsername: (args) => getAccountsByUsername(client, args),
    getUsernameByAddress: (args) => getUsernameByAddress(client, args),
    getNextAccountIndex: (args) => getNextAccountIndex(client, args),
    getNextAccountAddress: (args) => getNextAccountAddress(client, args),
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
