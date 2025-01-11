"use client";

import { type GetAccountReturnType, getAccount, watchAccount } from "@left-curve/dango-kit";
import type { AccountTypes, Config, ConfigParameter } from "@left-curve/types";
import { useConfig } from "./useConfig.js";
import { useSyncExternalStoreWithTracked } from "./useSyncExternalStoreWithTRacked.js";

export type UseAccountParameters<config extends Config = Config> = ConfigParameter<config>;

export type UseAccountReturnType<accountType extends AccountTypes = AccountTypes> =
  GetAccountReturnType<accountType>;

export function useAccount<
  accountType extends AccountTypes = AccountTypes,
  config extends Config = Config,
>(parameters: UseAccountParameters = {}): UseAccountReturnType<accountType> {
  const config = useConfig<config>(parameters);

  return useSyncExternalStoreWithTracked(
    (onChange) => watchAccount(config, { onChange }),
    () => getAccount<accountType, config>(config),
  );
}
