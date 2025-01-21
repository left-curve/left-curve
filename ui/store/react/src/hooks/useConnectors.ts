"use client";

import { type GetConnectorsReturnType, getConnectors } from "@left-curve/dango-sdk";
import type { ConfigParameter } from "@left-curve/types";

import { useConfig } from "./useConfig.js";

export type UseConnectorsParameters = ConfigParameter;

export type UseConnectorsReturnType = GetConnectorsReturnType;

export function useConnectors(parameters: UseConnectorsParameters = {}): UseConnectorsReturnType {
  const config = useConfig(parameters);
  const connectors = getConnectors(config);

  return connectors;
}
