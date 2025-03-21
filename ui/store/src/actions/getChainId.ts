import type { Config } from "../types/store.js";

export type GetChainIdReturnType<config extends Config = Config> = config["chains"][number]["id"];

export function getChainId<config extends Config>(config: config): GetChainIdReturnType<config> {
  return config.state.chainId;
}
