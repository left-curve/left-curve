import { type GetBlockExplorerParameters, getBlockExplorer } from "@leftcurve/connect-kit";
import type { Config, ConfigParameter, Prettify } from "@leftcurve/types";
import { useConfig } from "./useConfig.js";

export type UseBlockExplorerParameters<config extends Config = Config> = Prettify<
  GetBlockExplorerParameters & ConfigParameter<config>
>;

export type UseBlockExplorerReturnType = ReturnType<typeof getBlockExplorer>;

export function useBlockExplorer(parameters: UseBlockExplorerParameters = {}) {
  const config = useConfig(parameters);
  return getBlockExplorer(config, parameters);
}
