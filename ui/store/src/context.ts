"use client";

import { type PropsWithChildren, createContext, createElement } from "react";
import { Hydrate } from "./hydrate.js";
import { createConfig } from "./createConfig.js";
import { graphql } from "@left-curve/dango";
import { remote } from "./connectors/remote.js";

import { ConnectionStatus, type Config, type State } from "./types/store.js";
import type { WindowDangoStore } from "./remote.js";

declare let window: WindowDangoStore;

export const DangoStoreContext = createContext<Config | undefined>(undefined);

export type DangoStoreProviderProps = {
  config: Config;
  initialState?: State;
  reconnectOnMount?: boolean;
};

export const DangoStoreProvider: React.FC<React.PropsWithChildren<DangoStoreProviderProps>> = (
  parameters: PropsWithChildren<DangoStoreProviderProps>,
) => {
  const { children, config } = parameters;

  return createElement(
    Hydrate,
    parameters,
    createElement(DangoStoreContext.Provider, { value: config }, children),
  );
};

export const DangoRemoteProvider: React.FC<React.PropsWithChildren> = (parameters) => {
  const { children } = parameters;

  const chain = window.dango.chain;
  const connection = window.dango.connection;

  const config = createConfig({
    chain,
    transport: graphql(chain.urls.indexer, { batch: true }),
    coins: window.dango.coins,
    ssr: false,
    connectors: [remote()],
  });

  const connector = config.connectors.at(0)!;

  const initialState = connection
    ? {
        chainId: chain.id,
        isMipdLoaded: true,
        current: connector.uid,
        username: connection.account!.username,
        connectors: new Map([[connector.uid, { ...connection, connector }]]),
        status: ConnectionStatus.Connected,
      }
    : {
        chainId: chain.id,
        isMipdLoaded: true,
        current: null,
        username: undefined,
        connectors: new Map(),
        status: ConnectionStatus.Disconnected,
      };

  return createElement(
    Hydrate,
    { config, initialState, reconnectOnMount: false },
    createElement(DangoStoreContext.Provider, { value: config }, children),
  );
};
