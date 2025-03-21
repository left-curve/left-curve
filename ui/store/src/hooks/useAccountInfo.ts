import {
  type GetAccountInfoData,
  type GetAccountInfoErrorType,
  type GetAccountInfoOptions,
  type GetAccountInfoQueryFnData,
  type GetAccountInfoQueryKey,
  getAccountInfoQueryOptions,
} from "../handlers/getAccountInfo.js";

import { type QueryParameter, type UseQueryReturnType, useQuery } from "../query.js";
import { useConfig } from "./useConfig.js";

import type { Prettify } from "@left-curve/dango/types";
import type { Config, ConfigParameter } from "../types/store.js";

export type UseAccountInfoParameters<
  config extends Config = Config,
  selectData = GetAccountInfoData,
> = Prettify<
  GetAccountInfoOptions &
    ConfigParameter<config> &
    QueryParameter<
      GetAccountInfoQueryFnData,
      GetAccountInfoErrorType,
      selectData,
      GetAccountInfoQueryKey
    >
>;

export type UseAccountInfoReturnType<selectData = GetAccountInfoData> = UseQueryReturnType<
  selectData,
  GetAccountInfoErrorType
>;

export function useAccountInfo(parameters: UseAccountInfoParameters): UseAccountInfoReturnType {
  const { query } = parameters;

  const config = useConfig(parameters);
  const options = getAccountInfoQueryOptions(config, {
    ...parameters,
  });

  return useQuery({ ...query, ...options });
}
