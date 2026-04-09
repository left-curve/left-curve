import { toAccount } from "@left-curve/dango";
import { getAccountStatus, getUser } from "@left-curve/dango/actions";

import type { Address } from "@left-curve/dango/types";
import { type Config, ConnectionStatus, type StoreUser } from "../types/store.js";

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

  const userIndex = config.state.user?.index;

  const user = userIndex
    ? await getUser(client, { userIndexOrName: { index: userIndex } }).catch(() => undefined)
    : undefined;

  const accounts = user
    ? Object.entries(user.accounts).map(([accountIndex, address]) =>
        toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
      )
    : undefined;

  const connectors = new Map();
  for await (const {
    chainId,
    connector: _connector_,
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
      // The `connect` listener is attached once in setup() and is intentionally
      // never stripped — removing it during reconnect used to cause future
      // login attempts to silently drop the `connect` event.
      connector.emitter.on("change", config._internal.events.change);
      connector.emitter.on("disconnect", config._internal.events.disconnect);

      connectors.set(connector.uid, {
        keyHash,
        account: accounts?.find((a) => a.address === account.address) || accounts?.[0],
        chainId,
        accounts,
        connector,
      });
    } catch (_) {}
  }

  const userStatus = accounts
    ? await getAccountStatus(client, { address: accounts[0].address }).catch(() => undefined)
    : undefined;

  const userState: StoreUser | undefined =
    userIndex !== undefined
      ? {
          index: userIndex,
          username: user?.name ?? `User #${userIndex}`,
          status: userStatus,
        }
      : undefined;

  config.setState((x) => ({
    ...x,
    connectors,
    current,
    user: userState,
    status: connectors.size > 0 ? ConnectionStatus.Connected : ConnectionStatus.Disconnected,
  }));

  isReconnecting = false;
}
