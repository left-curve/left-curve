import { type Config, ConnectionStatus } from "../types/store.js";

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
    keyHash,
  } of config.state.connections.values()) {
    const connector = config.connectors.find(({ id }) => id === _connector_.id);
    const chain = config.chains.find(({ id }) => id === chainId);
    if (!connector || !chain) continue;

    try {
      connector.onConnect({ chainId, username });
      connector.emitter.off("connect", config._internal.events.connect);
      connector.emitter.on("change", config._internal.events.change);
      connector.emitter.on("disconnect", config._internal.events.disconnect);
      connectors.set(chainId, connector.uid);
      connections.set(connector.uid, {
        keyHash,
        account,
        chainId,
        accounts,
        connector,
        username,
      });
    } catch (_) {}
  }

  config.setState((x) => ({
    ...x,
    connections,
    connectors,
    status: connections.size > 0 ? ConnectionStatus.Connected : ConnectionStatus.Disconnected,
  }));

  isReconnecting = false;
}
