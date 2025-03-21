import {
  type GetBalancesData,
  type GetBalancesErrorType,
  type GetBalancesOptions,
  type GetBalancesQueryFnData,
  type GetBalancesQueryKey,
  getBalancesQueryOptions,
} from "../handlers/getBalances.js";

import { type QueryParameter, type UseQueryReturnType, useQuery } from "../query.js";
import { useConfig } from "./useConfig.js";

import type { Prettify } from "@left-curve/dango/types";
import type { Config, ConfigParameter } from "../types/store.js";

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
