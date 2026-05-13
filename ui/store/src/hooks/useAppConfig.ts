import {
  type GetAppConfigData,
  type GetAppConfigErrorType,
  type GetAppConfigOptions,
  type GetAppConfigQueryFnData,
  type GetAppConfigQueryKey,
  getAppConfigQueryOptions,
} from "../handlers/getAppConfig.js";

import { useSuspenseQuery } from "@tanstack/react-query";
import type { UseSuspenseQueryResult } from "@tanstack/react-query";
import type { QueryParameter } from "../query.js";
import { useConfig } from "./useConfig.js";

import type { Prettify } from "@left-curve/dango/types";
import type { Config, ConfigParameter } from "../types/store.js";

export type UseAppConfigParameters<
  config extends Config = Config,
  selectData = GetAppConfigData,
> = Prettify<
  GetAppConfigOptions &
    ConfigParameter<config> &
    QueryParameter<GetAppConfigQueryFnData, GetAppConfigErrorType, selectData, GetAppConfigQueryKey>
>;

export type UseAppConfigReturnType<selectData = GetAppConfigData> = UseSuspenseQueryResult<
  selectData,
  GetAppConfigErrorType
>;

export function useAppConfig(parameters: UseAppConfigParameters = {}): UseAppConfigReturnType {
  const { query } = parameters;

  const config = useConfig(parameters);
  const options = getAppConfigQueryOptions(config, {
    ...parameters,
  });

  return useSuspenseQuery({ ...query, ...options });
}
