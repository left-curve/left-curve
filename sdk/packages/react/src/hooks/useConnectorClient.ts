import type { Config, ConfigParameter, Prettify } from "@left-curve/types";
import { type QueryParameter, type UseQueryReturnType, useQuery } from "../query.js";
import { useConfig } from "./useConfig.js";

import {
  type GetConnectorClientData,
  type GetConnectorClientErrorType,
  type GetConnectorClientFnData,
  type GetConnectorClientOptions,
  type GetConnectorClientQueryKey,
  getConnectorClientQueryOptions,
} from "@left-curve/dango-sdk/handlers";

export type UseConnectorClientParameters<
  config extends Config = Config,
  selectData = GetConnectorClientData,
> = Prettify<
  GetConnectorClientOptions &
    ConfigParameter<config> &
    QueryParameter<
      GetConnectorClientFnData,
      GetConnectorClientErrorType,
      selectData,
      GetConnectorClientQueryKey
    >
>;

export type UseConnectorClientReturnType<selectData = GetConnectorClientData> = UseQueryReturnType<
  selectData,
  GetConnectorClientErrorType
>;

export function useConnectorClient(
  parameters: UseConnectorClientParameters = {},
): UseConnectorClientReturnType {
  const { query = {} } = parameters;

  const config = useConfig(parameters);
  const options = getConnectorClientQueryOptions(config, {
    ...parameters,
  });

  return useQuery({ ...query, ...options });
}
