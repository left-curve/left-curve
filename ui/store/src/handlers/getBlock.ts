import type { QueryOptions } from "@tanstack/query-core";

import {
  type GetBlockErrorType,
  type GetBlockParameters,
  type GetBlockReturnType,
  getBlock,
} from "../actions/getBlock.js";

import { type ScopeKeyParameter, filterQueryOptions } from "./query.js";

import type { Prettify } from "@left-curve/dango/types";
import type { Config } from "../types/store.js";

export type { GetBlockErrorType };

export type GetBlockOptions = Prettify<GetBlockParameters> & ScopeKeyParameter;

export function getBlockQueryOptions<config extends Config>(
  config: config,
  options: GetBlockOptions,
) {
  return {
    async queryFn({ queryKey }) {
      const { scopeKey: _, ...parameters } = queryKey[1];
      return getBlock(config, parameters);
    },
    queryKey: getBlockQueryKey(options),
  } as const satisfies QueryOptions<
    GetBlockQueryFnData,
    GetBlockErrorType,
    GetBlockData,
    GetBlockQueryKey
  >;
}

export type GetBlockQueryFnData = GetBlockReturnType;

export type GetBlockData = GetBlockQueryFnData;

export function getBlockQueryKey(options: GetBlockOptions) {
  return ["GetBlock", filterQueryOptions(options)] as const;
}

export type GetBlockQueryKey = ReturnType<typeof getBlockQueryKey>;
