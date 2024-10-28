import type { Config, ConfigParameter, Prettify } from "@leftcurve/types";
import { type QueryParameter, type UseQueryReturnType, useQuery } from "../query.js";
import { useConfig } from "./useConfig.js";

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

export function useBalances(
  parameters: Omit<UseBalancesParameters, "address"> & { address?: string },
): UseBalancesReturnType {
  const { query = {}, address } = parameters;

  const config = useConfig({ config: parameters.config });
  const options = getBalancesQueryOptions(config, {
    ...parameters,
    address: address as `0x${string}`,
  });

  return useQuery({ ...query, enabled: query.enabled || Boolean(address), ...options });
}
