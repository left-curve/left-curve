"use client";

import { useSyncExternalStoreWithTracked } from "./useSyncExternalStoreWithTRacked.js";

import { type GetAccountReturnType, getAccount } from "../actions/getAccount.js";
import { watchAccount } from "../actions/watchAccount.js";
import { useConfig } from "./useConfig.js";

import type { Config, ConfigParameter } from "../types/store.js";

export type UseAccountParameters<config extends Config = Config> = ConfigParameter<config>;

export type UseAccountReturnType = GetAccountReturnType;

export function useAccount<config extends Config = Config>(
  parameters: UseAccountParameters = {},
): UseAccountReturnType {
  const config = useConfig<config>(parameters);

  return useSyncExternalStoreWithTracked(
    (onChange) => watchAccount(config, { onChange }),
    () => getAccount<config>(config),
  );
}
