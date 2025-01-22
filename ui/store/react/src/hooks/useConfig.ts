"use client";

import { useContext } from "react";
import { DangoStoreContext } from "../context.js";

import type { Config } from "@left-curve/store/types";

export type UseConfigParameters<config extends Config = Config> = { config?: Config | config };

export type UseConfigReturnType<config extends Config = Config> = config;

export function useConfig<config extends Config = Config>(
  parameters: UseConfigParameters<config> = {},
): UseConfigReturnType<config> {
  const config = parameters.config ?? useContext(DangoStoreContext);
  if (!config) throw new Error("GrunnectProvider not found");
  return config as UseConfigReturnType<config>;
}
