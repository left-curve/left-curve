"use client";

import type { Config, State } from "@leftcurve/types";
import { type PropsWithChildren, createContext, createElement } from "react";
import { Hydrate } from "./hydrate.js";

export const GrunnectContext = createContext<Config | undefined>(undefined);

export type GrunnectProviderProps = {
  config: Config;
  initialState?: State;
  reconnectOnMount?: boolean;
};

export const GrunnectProvider: React.FC<React.PropsWithChildren<GrunnectProviderProps>> = (
  parameters: PropsWithChildren<GrunnectProviderProps>,
) => {
  const { children, config } = parameters;

  return createElement(
    Hydrate,
    parameters,
    createElement(GrunnectContext.Provider, { value: config }, children),
  );
};
