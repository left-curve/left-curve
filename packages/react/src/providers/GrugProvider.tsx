"use client";

import type { Config } from "@leftcurve/types";
import { type PropsWithChildren, createContext, createElement, useContext } from "react";

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

export function useGrugContext() {
  const context = useContext(GrugContext);
  if (!context) throw new Error("GrugProvider not found");
  return context;
}
