import { getAppConfig } from "../actions/getAppConfig.js";

import { type ScopeKeyParameter, filterQueryOptions } from "./query.js";

import type { Prettify } from "@left-curve/dango/types";
import type { QueryOptions } from "@tanstack/query-core";
import type { GetAppConfigErrorType, GetAppConfigReturnType } from "../actions/getAppConfig.js";
import type { Config } from "../types/store.js";

export type GetAppConfigOptions = Prettify<ScopeKeyParameter>;

export type { GetAppConfigErrorType };

export function getAppConfigQueryOptions<config extends Config>(
  config: config,
  options: GetAppConfigOptions,
) {
  return {
    async queryFn({ queryKey }) {
      const { scopeKey: _ } = queryKey[1];
      return getAppConfig(config);
    },
    queryKey: getAppConfigQueryKey(options),
  } as const satisfies QueryOptions<
    GetAppConfigQueryFnData,
    GetAppConfigErrorType,
    GetAppConfigData,
    GetAppConfigQueryKey
  >;
}

export type GetAppConfigQueryFnData = Awaited<GetAppConfigReturnType>;

export type GetAppConfigData = GetAppConfigQueryFnData;

export function getAppConfigQueryKey(options: GetAppConfigOptions) {
  return ["getAppConfig", filterQueryOptions(options)] as const;
}

export type GetAppConfigQueryKey = ReturnType<typeof getAppConfigQueryKey>;
