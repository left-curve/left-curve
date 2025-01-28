import type { QueryOptions } from "@tanstack/query-core";

import {
  type GetAccountInfoErrorType,
  type GetAccountInfoParameters,
  type GetAccountInfoReturnType,
  getAccountInfo,
} from "../actions/getAccountInfo.js";

import { type ScopeKeyParameter, filterQueryOptions } from "./query.js";

import type { Prettify } from "@left-curve/dango/types";
import type { Config } from "../types/store.js";

export type { GetAccountInfoErrorType };

export type GetAccountInfoOptions = Prettify<GetAccountInfoParameters & ScopeKeyParameter>;

export function getAccountInfoQueryOptions<config extends Config>(
  config: config,
  options: GetAccountInfoOptions,
) {
  return {
    async queryFn({ queryKey }) {
      const { scopeKey: _, ...parameters } = queryKey[1];
      return getAccountInfo(config, parameters);
    },
    queryKey: getAccountInfoQueryKey(options),
  } as const satisfies QueryOptions<
    GetAccountInfoQueryFnData,
    GetAccountInfoErrorType,
    GetAccountInfoData,
    GetAccountInfoQueryKey
  >;
}

export type GetAccountInfoQueryFnData = GetAccountInfoReturnType;

export type GetAccountInfoData = GetAccountInfoQueryFnData;

export function getAccountInfoQueryKey(options: GetAccountInfoOptions) {
  return ["getAccountInfo", filterQueryOptions(options)] as const;
}

export type GetAccountInfoQueryKey = ReturnType<typeof getAccountInfoQueryKey>;
