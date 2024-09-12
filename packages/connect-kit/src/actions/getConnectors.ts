import type { Config, Connector } from "@leftcurve/types";
import { assertDeepEqual } from "@leftcurve/utils";

export type GetConnectorsReturnType = readonly Connector[];

let previousConnectors: readonly Connector[] = [];

export function getConnectors(config: Config): GetConnectorsReturnType {
  const connectors = config.connectors;
  if (assertDeepEqual(previousConnectors, connectors)) return previousConnectors;
  previousConnectors = connectors;
  return connectors;
}
