"use client";

import { type PropsWithChildren, createContext, createElement } from "react";
import { Hydrate } from "./hydrate.js";
import type { Config, State } from "./types/store.js";

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
