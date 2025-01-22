import {
  type GetBlockData,
  type GetBlockErrorType,
  type GetBlockOptions,
  type GetBlockQueryFnData,
  type GetBlockQueryKey,
  getBlockQueryOptions,
} from "@left-curve/store";

import { type QueryParameter, type UseQueryReturnType, useQuery } from "../query.js";
import { useConfig } from "./useConfig.js";

import type { Config, ConfigParameter } from "@left-curve/store/types";
import type { Prettify } from "@left-curve/types";

export type UseBlockParameters<
  config extends Config = Config,
  selectData = GetBlockData,
> = Prettify<
  GetBlockOptions &
    ConfigParameter<config> &
    QueryParameter<GetBlockQueryFnData, GetBlockErrorType, selectData, GetBlockQueryKey>
>;

export type UseBlockReturnType<selectData = GetBlockData> = UseQueryReturnType<
  selectData,
  GetBlockErrorType
>;

export function useBlock(parameters: UseBlockParameters = {}): UseBlockReturnType {
  const { query = {} } = parameters;

  // TODO: Use watch block
  const config = useConfig(parameters);
  const options = getBlockQueryOptions(config, {
    ...parameters,
  });

  return useQuery({ ...query, ...options });
}
