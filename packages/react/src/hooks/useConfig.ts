"use client";

import type { Config } from "@leftcurve/types";
import { useContext } from "react";
import { GrugContext } from "~/providers/GrugProvider";

export type UseConfigParameters<config extends Config = Config> = { config?: Config | config };

export type UseConfigReturnType<config extends Config = Config> = config;

export function useConfig<config extends Config = Config>(
  parameters: UseConfigParameters<config> = {},
): UseConfigReturnType<config> {
  const config = parameters.config ?? useContext(GrugContext);
  if (!config) throw new Error("GrugProvider not found");
  return config as UseConfigReturnType<config>;
}
