import {
  type GetPublicClientParameters,
  type GetPublicClientReturnType,
  getPublicClient,
  watchPublicClient,
} from "@left-curve/connect-kit";

import { useSyncExternalStoreWithSelector } from "use-sync-external-store/shim/with-selector.js";
import { useConfig } from "./useConfig.js";

import type { Config, ConfigParameter, Prettify } from "@left-curve/types";

export type UsePublicClientParameters<config extends Config = Config> = Prettify<
  GetPublicClientParameters & ConfigParameter<config>
>;

export type UsePublicClientReturnType = GetPublicClientReturnType;

export function usePublicClient<config extends Config = Config>(
  parameters: UsePublicClientParameters<config> = {},
): UsePublicClientReturnType {
  const config = useConfig(parameters);

  return useSyncExternalStoreWithSelector(
    (onChange) => watchPublicClient(config, { onChange }),
    () => getPublicClient(config, parameters),
    () => getPublicClient(config, parameters),
    (x) => x,
    (a, b) => a?.uid === b?.uid,
  );
}
