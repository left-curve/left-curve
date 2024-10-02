import { publicActions } from "@leftcurve/sdk";
import type { PublicClient } from "@leftcurve/sdk/clients";
import type { Config } from "@leftcurve/types";

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
