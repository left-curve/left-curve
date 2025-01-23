import { type GetBlockExplorerParameters, getBlockExplorer } from "@left-curve/store";
import { useConfig } from "./useConfig.js";

import type { Config, ConfigParameter } from "@left-curve/store/types";
import type { Prettify } from "@left-curve/types";

export type UseBlockExplorerParameters<config extends Config = Config> = Prettify<
  GetBlockExplorerParameters & ConfigParameter<config>
>;

export type UseBlockExplorerReturnType = ReturnType<typeof getBlockExplorer>;

export function useBlockExplorer(parameters: UseBlockExplorerParameters = {}) {
  const config = useConfig(parameters);
  return getBlockExplorer(config, parameters);
}
