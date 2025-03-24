import { ConnectionStatus } from "../types/store.js";

import type { UID } from "@left-curve/dango/types";

import type { Connector } from "../types/connector.js";
import type { Config } from "../types/store.js";

export type DisconnectParameters = {
  connectorUId?: UID;
};

export type DisconnectReturnType = void;

export type DisconnectErrorType = Error;

export async function disconnect(
  config: Config,
  parameters: DisconnectParameters,
): Promise<DisconnectReturnType> {
  const { connectors } = config.state;
  const { connectorUId } = parameters;
  let connector: Connector | undefined;
  if (connectorUId) connector = connectors.get(connectorUId)?.connector;
  else connector = connectors.get(config.state.current ?? "")?.connector;

  if (connector) {
    await connector.disconnect();
    connectors.delete(connector.uid);
  }

  config.setState((x) => {
    if (connectors.size === 0) {
      return {
        ...x,
        connectors: new Map(),
        status: ConnectionStatus.Disconnected,
      };
    }
    return {
      ...x,
      connectors: new Map(connectors),
    };
  });
}
