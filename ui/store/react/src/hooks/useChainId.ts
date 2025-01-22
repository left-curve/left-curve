"use client";

import { useSyncExternalStore } from "react";

import { type GetChainIdReturnType, getChainId, watchChainId } from "@left-curve/store";
import { useConfig } from "./useConfig.js";

import type { Config, ConfigParameter } from "@left-curve/store/types";

export type UseChainIdParameters<config extends Config = Config> = ConfigParameter<config>;

export type UseChainIdReturnType<config extends Config = Config> = GetChainIdReturnType<config>;

export function useChainId<config extends Config = Config>(
  parameters: UseChainIdParameters<config> = {},
): UseChainIdReturnType<config> {
  const config = useConfig(parameters);

  return useSyncExternalStore(
    (onChange) => watchChainId(config, { onChange }),
    () => getChainId(config),
    () => getChainId(config),
  );
}
