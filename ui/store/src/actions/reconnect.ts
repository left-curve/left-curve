import { getAccountStatus, getUsernameByIndex } from "@left-curve/dango/actions";
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

  const client = config.getClient();

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
    const isAuthorized = await connector.isAuthorized?.();
    if (!isAuthorized) continue;

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

  const userIndexAndName = await (async () => {
    if (!config.state.userIndexAndName) return;
    const { index } = config.state.userIndexAndName;
    const name = await getUsernameByIndex(client, { index });
    return { index, name: name || `User #${index}` };
  })();

  const userStatus = await (async () => {
    if (!config.state.userStatus || !config.state.current) return;
    const address = config.state.connectors.get(config.state.current)?.account?.address;
    if (!address) return;
    const status = await getAccountStatus(client, { address }).catch(() => undefined);
    return status;
  })();

  config.setState((x) => ({
    ...x,
    connectors,
    current,
    userIndexAndName,
    userStatus,
    status: connectors.size > 0 ? ConnectionStatus.Connected : ConnectionStatus.Disconnected,
  }));

  isReconnecting = false;
}
