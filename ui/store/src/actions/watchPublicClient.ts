import type { Config } from "../types/store.js";
import { type GetPublicClientReturnType, getPublicClient } from "./getPublicClient.js";

export type WatchPublicClientParameters = {
  onChange(
    publicClient: GetPublicClientReturnType,
    prevPublicClient: GetPublicClientReturnType,
  ): void;
};

export type WatchPublicClientReturnType = () => void;

export function watchPublicClient<config extends Config>(
  config: config,
  parameters: WatchPublicClientParameters,
): WatchPublicClientReturnType {
  const { onChange } = parameters;
  return config.subscribe(() => getPublicClient(config), onChange, {
    equalityFn(a, b) {
      return a?.uid === b?.uid;
    },
  });
}
