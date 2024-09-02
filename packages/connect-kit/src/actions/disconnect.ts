import type { ChainId, Config, Connector, ConnectorId, OneRequired } from "@leftcurve/types";

export type DisconnectParameters = OneRequired<
  {
    connectorId?: ConnectorId;
    chainId?: ChainId;
  },
  "connectorId",
  "chainId"
>;

export type DisconnectReturnType = void;

export type DisconnectErrorType = Error;

export async function disconnect(
  config: Config,
  parameters: DisconnectParameters,
): Promise<DisconnectReturnType> {
  const { connections, connectors } = config.state;
  const { chainId, connectorId } = parameters;
  let connector: Connector | undefined;
  if (connectorId) connector = connections.get(connectorId)?.connector;
  else {
    const connectorId = connectors.get(chainId!);
    if (!connectorId) throw new Error("No connector found for chain");
    connector = connections.get(connectorId)?.connector;
  }

  if (connector) {
    await connector.disconnect();
    connections.delete(connector.uid);
    for (const [k, v] of connectors) {
      if (v === connector.uid) {
        connectors.delete(k);
        break;
      }
    }
  }

  config.setState((x) => {
    if (connections.size === 0) {
      return {
        ...x,
        connections: new Map(),
        connectors: new Map(),
        status: "disconnected",
      };
    }
    return {
      ...x,
      connections: new Map(connections),
      connectors: new Map(connectors),
    };
  });
}
