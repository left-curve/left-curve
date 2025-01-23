import { assertDeepEqual } from "@left-curve/utils";

import type { Connector } from "../types/connector.js";
import type { Config } from "../types/store.js";

export type GetConnectorsReturnType = readonly Connector[];

let previousConnectors: readonly Connector[] = [];

export function getConnectors(config: Config): GetConnectorsReturnType {
  const connectors = config.connectors;
  if (assertDeepEqual(previousConnectors, connectors)) return previousConnectors;
  previousConnectors = connectors;
  return connectors;
}
