import type {
  AppConfigResponse,
  Chain,
  Client,
  JsonValue,
  Signer,
  Transport,
} from "@left-curve/types";

import {
  type GetBalanceParameters,
  type GetBalanceReturnType,
  getBalance,
} from "./public/getBalance.js";

import {
  type GetBalancesParameters,
  type GetBalancesReturnType,
  getBalances,
} from "./public/getBalances.js";

import {
  type GetSupplyParameters,
  type GetSupplyReturnType,
  getSupply,
} from "./public/getSupply.js";

import {
  type GetSuppliesParameters,
  type GetSuppliesReturnType,
  getSupplies,
} from "./public/getSupplies.js";

import { type GetCodeParameters, type GetCodeReturnType, getCode } from "./public/getCode.js";

import { type GetCodesParameters, type GetCodesReturnType, getCodes } from "./public/getCodes.js";

import {
  type GetChainInfoParameters,
  type GetChainInfoReturnType,
  getChainInfo,
} from "./public/getChainInfo.js";

import { type QueryAppParameters, type QueryAppReturnType, queryApp } from "./public/queryApp.js";

import {
  type QueryWasmRawParameters,
  type QueryWasmRawReturnType,
  queryWasmRaw,
} from "./public/queryWasmRaw.js";

import {
  type QueryWasmSmartParameters,
  type QueryWasmSmartReturnType,
  queryWasmSmart,
} from "./public/queryWasmSmart.js";

import {
  type RegisterUserParameters,
  type RegisterUserReturnType,
  registerUser,
} from "./public/registerUser.js";

import { type SimulateParameters, type SimulateReturnType, simulate } from "./public/simulate.js";

import {
  type ComputeAddressParameters,
  type ComputeAddressReturnType,
  computeAddress,
} from "./public/computeAddress.js";

import {
  type GetAppConfigParameters,
  type GetAppConfigReturnType,
  getAppConfig,
} from "./public/getAppConfig.js";

import {
  type GetAccountTypeCodeHashParameters,
  type GetAccountTypeCodeHashReturnType,
  getAccountTypeCodeHash,
} from "./public/getAccountTypeCodeHash.js";

import {
  type GetAccountTypeCodeHashesParameters,
  type GetAccountTypeCodeHashesReturnType,
  getAccountTypeCodeHashes,
} from "./public/getAccountTypeCodeHashes.js";

import {
  type GetUsersByKeyHashReturnType,
  type GetUsersByKeyhashParameters,
  getUsersByKeyHash,
} from "./public/getUsersByKeyHash.js";

import {
  type GetKeysByUsernameParameters,
  type GetKeysByUsernameReturnType,
  getKeysByUsername,
} from "./public/getKeysByUsername.js";

import { type GetKeyParameters, type GetKeyReturnType, getKey } from "./public/getKey.js";

import { type GetKeysParameters, type GetKeysReturnType, getKeys } from "./public/getKeys.js";

import {
  type GetAccountsByUsernameParameters,
  type GetAccountsByUsernameReturnType,
  getAccountsByUsername,
} from "./public/getAccountsByUsername.js";

import {
  type GetNextAccountIndexParameters,
  type GetNextAccountIndexReturnType,
  getNextAccountIndex,
} from "./public/getNextAccountIndex.js";

import {
  type GetNextAccountAddressParameters,
  type GetNextAccountAddressReturnType,
  getNextAccountAddress,
} from "./public/getNextAccountAddress.js";

import {
  type GetContractInfoParameters,
  type GetContractInfoReturnType,
  getContractInfo,
} from "./public/getContractInfo.js";

import {
  type GetContractsInfoParameters,
  type GetContractsInfoReturnType,
  getContractsInfo,
} from "./public/getContractsInfo.js";

import {
  type GetDepositParameters,
  type GetDepositReturnType,
  getDeposit,
} from "./public/getDeposit.js";

import {
  type GetDepositsParameters,
  type GetDepositsReturnType,
  getDeposits,
} from "./public/getDeposits.js";

import {
  type GetAccountInfoParameters,
  type GetAccountInfoReturnType,
  getAccountInfo,
} from "./public/getAccountInfo.js";

import {
  type GetAllAccountInfoParameters,
  type GetAllAccountInfoReturnType,
  getAllAccountInfo,
} from "./public/getAllAccountInfo.js";

import { type GetUserParameters, type GetUserReturnType, getUser } from "./public/getUser.js";

export type PublicActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = undefined,
> = {
  getBalance: (args: GetBalanceParameters) => GetBalanceReturnType;
  getBalances: (args: GetBalancesParameters) => GetBalancesReturnType;
  getDeposit: (args: GetDepositParameters) => GetDepositReturnType;
  getDeposits: (args: GetDepositsParameters) => GetDepositsReturnType;
  getSupply: (args: GetSupplyParameters) => GetSupplyReturnType;
  getSupplies: (args?: GetSuppliesParameters) => GetSuppliesReturnType;
  getCode: (args: GetCodeParameters) => GetCodeReturnType;
  getCodes: (args?: GetCodesParameters) => GetCodesReturnType;
  getChainInfo: (args?: GetChainInfoParameters) => GetChainInfoReturnType;
  getAppConfig: <value extends AppConfigResponse>(
    args?: GetAppConfigParameters,
  ) => GetAppConfigReturnType<value>;
  getAccountTypeCodeHash: (
    args: GetAccountTypeCodeHashParameters,
  ) => GetAccountTypeCodeHashReturnType;
  getAccountTypeCodeHashes: (
    args?: GetAccountTypeCodeHashesParameters,
  ) => GetAccountTypeCodeHashesReturnType;
  getUser: (args: GetUserParameters) => GetUserReturnType;
  getAccountInfo: (args: GetAccountInfoParameters) => GetAccountInfoReturnType;
  getAllAccountInfo: (args: GetAllAccountInfoParameters) => GetAllAccountInfoReturnType;
  getUsersByKeyHash: (args: GetUsersByKeyhashParameters) => GetUsersByKeyHashReturnType;
  getKey: (args: GetKeyParameters) => GetKeyReturnType;
  getKeys: (args: GetKeysParameters) => GetKeysReturnType;
  getKeysByUsername: (args: GetKeysByUsernameParameters) => GetKeysByUsernameReturnType;
  getAccountsByUsername: (args: GetAccountsByUsernameParameters) => GetAccountsByUsernameReturnType;
  getNextAccountIndex: (args: GetNextAccountIndexParameters) => GetNextAccountIndexReturnType;
  getNextAccountAddress: (args: GetNextAccountAddressParameters) => GetNextAccountAddressReturnType;
  getContractInfo: (args: GetContractInfoParameters) => GetContractInfoReturnType;
  getContractsInfo: (args?: GetContractsInfoParameters) => GetContractsInfoReturnType;
  queryApp: (args: QueryAppParameters) => QueryAppReturnType;
  queryWasmRaw: (args: QueryWasmRawParameters) => QueryWasmRawReturnType;
  queryWasmSmart: <value extends JsonValue>(
    args: QueryWasmSmartParameters,
  ) => QueryWasmSmartReturnType<value>;
  registerUser: (args: RegisterUserParameters) => RegisterUserReturnType;
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
    getBalance: (args) => getBalance(client, args),
    getBalances: (args) => getBalances(client, args),
    getDeposit: (args) => getDeposit(client, args),
    getDeposits: (args) => getDeposits(client, args),
    getSupply: (args) => getSupply(client, args),
    getSupplies: (args) => getSupplies(client, args),
    getCode: (args) => getCode(client, args),
    getCodes: (args) => getCodes(client, args),
    getChainInfo: (args) => getChainInfo(client, args),
    getUser: (args) => getUser(client, args),
    getAccountTypeCodeHash: (args) => getAccountTypeCodeHash(client, args),
    getAccountTypeCodeHashes: (args) => getAccountTypeCodeHashes(client, args),
    getAccountInfo: (args) => getAccountInfo(client, args),
    getAllAccountInfo: (args) => getAllAccountInfo(client, args),
    getUsersByKeyHash: (args) => getUsersByKeyHash(client, args),
    getKey: (args) => getKey(client, args),
    getKeys: (args) => getKeys(client, args),
    getKeysByUsername: (args) => getKeysByUsername(client, args),
    getAccountsByUsername: (args) => getAccountsByUsername(client, args),
    getNextAccountIndex: (args) => getNextAccountIndex(client, args),
    getNextAccountAddress: (args) => getNextAccountAddress(client, args),
    getContractInfo: (args) => getContractInfo(client, args),
    getContractsInfo: (args) => getContractsInfo(client, args),
    queryApp: (args) => queryApp(client, args),
    queryWasmRaw: (args) => queryWasmRaw(client, args),
    queryWasmSmart: (args) => queryWasmSmart(client, args),
    registerUser: (args) => registerUser(client, args),
    simulate: (args) => simulate(client, args),
    computeAddress: (args) => computeAddress(args),
  };
}
