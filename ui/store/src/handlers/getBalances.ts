import type { QueryOptions } from "@tanstack/query-core";

import {
  type GetBalanceParameters,
  type GetBalancesErrorType,
  type GetBalancesReturnType,
  getBalances,
} from "../actions/getBalances.js";

import { type ScopeKeyParameter, filterQueryOptions } from "./query.js";

import type { Prettify } from "@left-curve/dango/types";
import type { Config } from "../types/store.js";

export type { GetBalancesErrorType };

export type GetBalancesOptions = Prettify<GetBalanceParameters & ScopeKeyParameter>;

export function getBalancesQueryOptions<config extends Config>(
  config: config,
  options: GetBalancesOptions,
) {
  return {
    async queryFn({ queryKey }) {
      const { scopeKey: _, ...parameters } = queryKey[1];
      return getBalances(config, parameters);
    },
    queryKey: getBalancesQueryKey(options),
  } as const satisfies QueryOptions<
    GetBalancesQueryFnData,
    GetBalancesErrorType,
    GetBalancesData,
    GetBalancesQueryKey
  >;
}

export type GetBalancesQueryFnData = GetBalancesReturnType;

export type GetBalancesData = GetBalancesQueryFnData;

export function getBalancesQueryKey(options: GetBalancesOptions) {
  return ["getBalances", filterQueryOptions(options)] as const;
}

export type GetBalancesQueryKey = ReturnType<typeof getBalancesQueryKey>;
