"use client";

import type { Config } from "@leftcurve/types";
import { type PropsWithChildren, createContext, createElement } from "react";

export const GrugContext = createContext<Config | undefined>(undefined);

export type GrugProviderProps = {
  config: Config;
};

export const GrugProvider: React.FC<React.PropsWithChildren<GrugProviderProps>> = (
  parameters: PropsWithChildren<GrugProviderProps>,
) => {
  const { children, config } = parameters;

  return createElement(GrugContext.Provider, { value: config }, children);
};
