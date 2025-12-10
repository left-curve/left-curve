import type { QueryOptions } from "@tanstack/query-core";

import {
  type GetConnectorClientErrorType,
  type GetConnectorClientParameters,
  type GetConnectorClientReturnType,
  getConnectorClient,
} from "../actions/getConnectorClient.js";

import { type ScopeKeyParameter, filterQueryOptions } from "./query.js";

import type { ExactPartial, Prettify } from "@left-curve/dango/types";
import type { Config } from "../types/store.js";

export type { GetConnectorClientErrorType };

export type GetConnectorClientOptions = Prettify<
  ExactPartial<GetConnectorClientParameters> & ScopeKeyParameter
>;

export function getConnectorClientQueryOptions<config extends Config>(
  config: config,
  options: GetConnectorClientOptions = {},
) {
  return {
    gcTime: 0,
    async queryFn({ queryKey }) {
      const { scopeKey: _, scopeKey: _s, ...parameters } = queryKey[1];
      return getConnectorClient(config, parameters);
    },
    queryKey: getConnectorClientQueryKey(config, options),
  } as const satisfies QueryOptions<
    GetConnectorClientFnData,
    GetConnectorClientErrorType,
    GetConnectorClientData,
    GetConnectorClientQueryKey
  >;
}

export type GetConnectorClientFnData = GetConnectorClientReturnType;

export type GetConnectorClientData = GetConnectorClientFnData;

export function getConnectorClientQueryKey(
  config: Config,
  options: GetConnectorClientOptions = {},
) {
  const { connectorUId, ...parameters } = options;
  return [
    "connectorClient",
    { ...filterQueryOptions(parameters), connectorUId, state: config.state },
  ] as const;
}

export type GetConnectorClientQueryKey = ReturnType<typeof getConnectorClientQueryKey>;
