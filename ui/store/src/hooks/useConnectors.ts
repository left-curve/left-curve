"use client";

import { type GetConnectorsReturnType, getConnectors } from "../actions/getConnectors.js";
import { useConfig } from "./useConfig.js";

import type { ConfigParameter } from "../types/store.js";

export type UseConnectorsParameters = ConfigParameter;

export type UseConnectorsReturnType = GetConnectorsReturnType;

export function useConnectors(parameters: UseConnectorsParameters = {}): UseConnectorsReturnType {
  const config = useConfig(parameters);
  const connectors = getConnectors(config);

  return connectors;
}
