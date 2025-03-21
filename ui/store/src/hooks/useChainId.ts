"use client";

import { useSyncExternalStore } from "react";

import { type GetChainIdReturnType, getChainId } from "../actions/getChainId.js";
import { watchChainId } from "../actions/watchChainId.js";

import { useConfig } from "./useConfig.js";

import type { Config, ConfigParameter } from "../types/store.js";

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
