import type { Config, Connector, ConnectorUId } from "@leftcurve/types";

export type GetConnectorParameters = {
  connectorUId?: ConnectorUId;
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
