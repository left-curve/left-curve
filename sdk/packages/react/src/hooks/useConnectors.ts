"use client";

import { type GetConnectorsReturnType, getConnectors } from "@leftcurve/connect-kit";
import type { ConfigParameter } from "@leftcurve/types";

import { useConfig } from "./useConfig";

export type UseConnectorsParameters = ConfigParameter;

export type UseConnectorsReturnType = GetConnectorsReturnType;

export function useConnectors(parameters: UseConnectorsParameters = {}): UseConnectorsReturnType {
  const config = useConfig(parameters);
  const connectors = getConnectors(config);

  return connectors;
}
