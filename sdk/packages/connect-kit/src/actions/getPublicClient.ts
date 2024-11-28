import { publicActions } from "@left-curve/sdk";
import type { PublicClient } from "@left-curve/sdk/clients";
import type { Config } from "@left-curve/types";

export type GetPublicClientParameters = {
  chainId?: string;
};

export type GetPublicClientReturnType = PublicClient;

export type GetPublicClientErrorType = Error;

export function getPublicClient<config extends Config>(
  config: config,
  parameters: GetPublicClientParameters = {},
): GetPublicClientReturnType {
  return config.getClient(parameters).extend(publicActions);
}
