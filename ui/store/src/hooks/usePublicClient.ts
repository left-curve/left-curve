import { type GetPublicClientReturnType, getPublicClient } from "../actions/getPublicClient.js";
import { watchPublicClient } from "../actions/watchPublicClient.js";

import { useSyncExternalStoreWithSelector } from "use-sync-external-store/shim/with-selector.js";
import { useConfig } from "./useConfig.js";

import type { Prettify } from "@left-curve/dango/types";
import type { Config, ConfigParameter } from "../types/store.js";

export type UsePublicClientParameters<config extends Config = Config> = Prettify<
  ConfigParameter<config>
>;

export type UsePublicClientReturnType = GetPublicClientReturnType;

export function usePublicClient<config extends Config = Config>(
  parameters: UsePublicClientParameters<config> = {},
): UsePublicClientReturnType {
  const config = useConfig(parameters);

  return useSyncExternalStoreWithSelector(
    (onChange) => watchPublicClient(config, { onChange }),
    () => getPublicClient(config),
    () => getPublicClient(config),
    (x) => x,
    (a, b) => a?.uid === b?.uid,
  );
}
