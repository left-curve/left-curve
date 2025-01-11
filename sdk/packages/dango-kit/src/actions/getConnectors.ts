import type { Config, Connector } from "@left-curve/types";
import { assertDeepEqual } from "@left-curve/utils";

export type GetConnectorsReturnType = readonly Connector[];

let previousConnectors: readonly Connector[] = [];

export function getConnectors(config: Config): GetConnectorsReturnType {
  const connectors = config.connectors;
  if (assertDeepEqual(previousConnectors, connectors)) return previousConnectors;
  previousConnectors = connectors;
  return connectors;
}
