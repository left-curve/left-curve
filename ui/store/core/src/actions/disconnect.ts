import { ConnectionStatus } from "../types/store.js";

import type { ChainId, OneRequired, UID } from "@left-curve/dango/types";

import type { Connector } from "../types/connector.js";
import type { Config } from "../types/store.js";

export type DisconnectParameters = OneRequired<
  {
    connectorUId?: UID;
    chainId?: ChainId;
  },
  "connectorUId",
  "chainId"
>;

export type DisconnectReturnType = void;

export type DisconnectErrorType = Error;

export async function disconnect(
  config: Config,
  parameters: DisconnectParameters,
): Promise<DisconnectReturnType> {
  const { connections, connectors } = config.state;
  const { chainId, connectorUId } = parameters;
  let connector: Connector | undefined;
  if (connectorUId) connector = connections.get(connectorUId)?.connector;
  else {
    const connectorUId = connectors.get(chainId!);
    if (!connectorUId) throw new Error("No connector found for chain");
    connector = connections.get(connectorUId)?.connector;
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
        status: ConnectionStatus.Disconnected,
      };
    }
    return {
      ...x,
      connections: new Map(connections),
      connectors: new Map(connectors),
    };
  });
}
