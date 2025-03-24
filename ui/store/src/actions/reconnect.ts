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
    status: x.connectors.size > 0 ? ConnectionStatus.Reconnecting : ConnectionStatus.Disconnected,
  }));

  let current = config.state.current;

  const connectors = new Map();
  for (const {
    chainId,
    connector: _connector_,
    accounts,
    account,
    keyHash,
  } of config.state.connectors.values()) {
    const connector = config.connectors.find(({ id }) => id === _connector_.id);
    const chain = chainId === config.state.chainId ? config.chain : undefined;
    if (!connector || !chain) continue;
    if (_connector_.uid === config.state.current) current = connector.uid;

    try {
      connector.emitter.off("connect", config._internal.events.connect);
      connector.emitter.on("change", config._internal.events.change);
      connector.emitter.on("disconnect", config._internal.events.disconnect);
      connectors.set(connector.uid, {
        keyHash,
        account,
        chainId,
        accounts,
        connector,
      });
    } catch (_) {}
  }

  config.setState((x) => ({
    ...x,
    connectors,
    current,
    username: config.state.username,
    status: connectors.size > 0 ? ConnectionStatus.Connected : ConnectionStatus.Disconnected,
  }));

  isReconnecting = false;
}
