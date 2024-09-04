"use client";

import type { Config } from "@leftcurve/types";
import { useGrugContext } from "~/providers";

export type UseConfigParameters<config extends Config = Config> = { config?: Config | config };

export type UseConfigReturnType<config extends Config = Config> = config;

export function useConfig<config extends Config = Config>(
  parameters: UseConfigParameters<config> = {},
): UseConfigReturnType<config> {
  const config = parameters.config ?? useGrugContext();
  if (!config) throw new Error("GrugProvider not found");
  return config as UseConfigReturnType<config>;
}
