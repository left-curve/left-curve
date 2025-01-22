"use client";

import type { Config, State } from "@left-curve/store/types";
import { type PropsWithChildren, createContext, createElement } from "react";
import { Hydrate } from "./hydrate.js";

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
