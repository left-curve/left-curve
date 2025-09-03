"use client";

import { type PropsWithChildren, createContext, createElement } from "react";
import { Hydrate } from "./hydrate.js";
import { createConfig } from "./createConfig.js";
import { graphql } from "@left-curve/dango";
import { remote } from "./connectors/remote.js";

import type { Config, State } from "./types/store.js";
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

  const config = createConfig({
    chain,
    transport: graphql(chain.urls.indexer, { batch: true }),
    coins: window.dango.coins,
    ssr: false,
    connectors: [remote()],
  });

  return createElement(
    Hydrate,
    { config, reconnectOnMount: false },
    createElement(DangoStoreContext.Provider, { value: config }, children),
  );
};
