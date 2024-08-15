import type { Account, Chain, Client, Transport } from "@leftcurve/types";

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

export type PublicActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = Account | undefined,
> = {
  getBalance: (args: GetBalanceParameters) => GetBalanceReturnType;
  getBalances: (args: GetBalancesParameters) => GetBalancesReturnType;
  getSupply: (args: GetSupplyParameters) => GetSupplyReturnType;
  getSupplies: (args: GetSuppliesParameters) => GetSuppliesReturnType;
  getCode: (args: GetCodeParameters) => GetCodeReturnType;
  getCodes: (args: GetCodesParameters) => GetCodesReturnType;
  getChainInfo: (args: GetChainInfoParameters) => GetChainInfoReturnType;
  queryApp: (args: QueryAppParameters) => QueryAppReturnType;
  // biome-ignore lint/suspicious/noExplicitAny: It could be any time
  queryWasmRaw: <value extends any | undefined>(
    args: QueryWasmRawParameters,
  ) => QueryWasmRawReturnType<value>;
  // biome-ignore lint/suspicious/noExplicitAny: It could be any time
  queryWasmSmart: <value extends any | undefined>(
    args: QueryWasmSmartParameters,
  ) => QueryWasmSmartReturnType<value>;
  createAccount: (args: CreateAccountParameters) => CreateAccountReturnType;
  simulate: (args: SimulateParameters) => SimulateReturnType;
};

export function publicActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = Account | undefined,
>(client: Client<transport, chain, account>): PublicActions<transport, chain, account> {
  return {
    getBalance: (args) => getBalance(client, args),
    getBalances: (args) => getBalances(client, args),
    getSupply: (args) => getSupply(client, args),
    getSupplies: (args) => getSupplies(client, args),
    getCode: (args) => getCode(client, args),
    getCodes: (args) => getCodes(client, args),
    getChainInfo: (args) => getChainInfo(client, args),
    queryApp: (args) => queryApp(client, args),
    queryWasmRaw: (args) => queryWasmRaw(client, args),
    queryWasmSmart: (args) => queryWasmSmart(client, args),
    createAccount: (args) => createAccount(client, args),
    simulate: (args) => simulate(client, args),
  };
}
