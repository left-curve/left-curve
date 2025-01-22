"use client";

import { useSyncExternalStoreWithTracked } from "./useSyncExternalStoreWithTRacked.js";

import { type GetAccountReturnType, getAccount, watchAccount } from "@left-curve/store";
import { useConfig } from "./useConfig.js";

import type { AccountTypes } from "@left-curve/dango/types";
import type { Config, ConfigParameter } from "@left-curve/store/types";

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
