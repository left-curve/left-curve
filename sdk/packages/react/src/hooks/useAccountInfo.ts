import type { Config, ConfigParameter, Prettify } from "@left-curve/types";
import { type QueryParameter, type UseQueryReturnType, useQuery } from "../query.js";
import { useConfig } from "./useConfig.js";

import {
  type GetAccountInfoData,
  type GetAccountInfoErrorType,
  type GetAccountInfoOptions,
  type GetAccountInfoQueryFnData,
  type GetAccountInfoQueryKey,
  getAccountInfoQueryOptions,
} from "@left-curve/connect-kit/handlers";

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
