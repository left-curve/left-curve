import type { SignerClient } from "@leftcurve/sdk/clients";
import type { Config, ConnectorUId } from "@leftcurve/types";

export type GetConnectorClientParameters = {
  connectorUId?: ConnectorUId;
};

export type GetConnectorClientReturnType = SignerClient;

export type GetConnectorClientErrorType = Error;

export async function getConnectorClient<config extends Config>(
  config: config,
  parameters: GetConnectorClientParameters = {},
): Promise<GetConnectorClientReturnType> {
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

  return await connection.connector.getClient();
}
