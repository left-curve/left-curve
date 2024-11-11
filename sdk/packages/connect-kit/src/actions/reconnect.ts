import { type Config, ConnectionStatus } from "@leftcurve/types";

export type ReconnectReturnType = void;

export type ReconnectErrorType = Error;

let isReconnecting = false;

export async function reconnect<config extends Config>(
  config: config,
): Promise<ReconnectReturnType> {
  if (isReconnecting) return;
  isReconnecting = true;

  config.setState((x) => ({
    ...x,
    status: x.connections.size > 0 ? ConnectionStatus.Reconnecting : ConnectionStatus.Disconnected,
  }));

  const connections = new Map();
  const connectors = new Map();
  for (const {
    chainId,
    connector: _connector_,
    username,
    accounts,
    account,
  } of config.state.connections.values()) {
    const connector = config.connectors.find(({ id }) => id === _connector_.id);
    if (!connector) continue;
    try {
      connector.connect({ chainId, username });
      connectors.set(chainId, connector.uid);
      connections.set(connector.uid, {
        account,
        chainId,
        accounts,
        connector,
        username,
      });
    } catch (_) {}

    config.setState((x) => ({
      ...x,
      connections,
      connectors,
      status: connections.size > 0 ? ConnectionStatus.Connected : ConnectionStatus.Disconnected,
    }));
  }

  isReconnecting = false;
}
