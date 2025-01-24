import type { UID } from "@left-curve/dango/types";

import type { Connector } from "../types/connector.js";
import type { Config } from "../types/store.js";

export type GetConnectorParameters = {
  connectorUId?: UID;
};

export type GetConnectorReturnType = Connector;

export type GetConnectorErrorType = Error;

export function getConnector<config extends Config>(
  config: config,
  parameters: GetConnectorParameters = {},
): GetConnectorReturnType {
  const { connectorUId } = parameters;
  const connection = (() => {
    if (connectorUId) {
      return config.state.connections.get(connectorUId);
    }

    const UId = config.state.connectors.get(config.state.chainId);
    if (!UId) throw new Error("No connector found for current chain");
    return config.state.connections.get(UId);
  })();

  if (!connection) throw new Error("No connection found");

  return connection.connector;
}
