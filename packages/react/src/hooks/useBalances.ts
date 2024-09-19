import type { Config, ConfigParameter, Prettify } from "@leftcurve/types";
import { type QueryParameter, type UseQueryReturnType, useQuery } from "~/utils/query";
import { useConfig } from "./useConfig";

import {
  type GetBalancesData,
  type GetBalancesErrorType,
  type GetBalancesOptions,
  type GetBalancesQueryFnData,
  type GetBalancesQueryKey,
  getBalancesQueryOptions,
} from "@leftcurve/connect-kit/handlers";

export type UseBalancesParameters<
  config extends Config = Config,
  selectData = GetBalancesData,
> = Prettify<
  GetBalancesOptions &
    ConfigParameter<config> &
    QueryParameter<GetBalancesQueryFnData, GetBalancesErrorType, selectData, GetBalancesQueryKey>
>;

export type UseBalancesReturnType<selectData = GetBalancesData> = UseQueryReturnType<
  selectData,
  GetBalancesErrorType
>;

export function useBalances(parameters: UseBalancesParameters): UseBalancesReturnType {
  const { query = {} } = parameters;

  const config = useConfig(parameters);
  const options = getBalancesQueryOptions(config, {
    ...parameters,
  });

  return useQuery({ ...query, ...options });
}
