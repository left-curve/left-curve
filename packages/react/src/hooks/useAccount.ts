"use client";

import { type GetAccountReturnType, getAccount, watchAccount } from "@leftcurve/connect-kit";
import type { Config, ConfigParameter } from "@leftcurve/types";
import { useConfig } from "./useConfig";
import { useSyncExternalStoreWithTracked } from "./useSyncExternalStoreWithTRacked";

export type UseAccountParameters<config extends Config = Config> = ConfigParameter<config>;

export type UseAccountReturnType = GetAccountReturnType;

export function useAccount<config extends Config = Config>(
  parameters: UseAccountParameters<config> = {},
): UseAccountReturnType {
  const config = useConfig(parameters);

  return useSyncExternalStoreWithTracked(
    (onChange) => watchAccount(config, { onChange }),
    () => getAccount(config),
  );
}
