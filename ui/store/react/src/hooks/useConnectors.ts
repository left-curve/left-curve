"use client";

import { type GetConnectorsReturnType, getConnectors } from "@left-curve/store";
import { useConfig } from "./useConfig.js";

import type { ConfigParameter } from "@left-curve/store/types";

export type UseConnectorsParameters = ConfigParameter;

export type UseConnectorsReturnType = GetConnectorsReturnType;

export function useConnectors(parameters: UseConnectorsParameters = {}): UseConnectorsReturnType {
  const config = useConfig(parameters);
  const connectors = getConnectors(config);

  return connectors;
}
