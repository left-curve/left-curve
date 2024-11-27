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
} from "@left-curve/connect-kit/handlers";

export type UseSigningClientParameters<
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

export type UseSigningClientReturnType<selectData = GetConnectorClientData> = UseQueryReturnType<
  selectData,
  GetConnectorClientErrorType
>;

export function useSigningClient(
  parameters: UseSigningClientParameters = {},
): UseSigningClientReturnType {
  const { query = {} } = parameters;

  const config = useConfig(parameters);
  const options = getConnectorClientQueryOptions(config, {
    ...parameters,
  });

  return useQuery({ ...query, ...options });
}
