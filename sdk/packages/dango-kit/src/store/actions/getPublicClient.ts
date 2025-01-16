import { publicActions } from "../../actions/publicActions.js";
import type { Config } from "../../types/store.js";

export type GetPublicClientParameters = {
  chainId?: string;
};

export type GetPublicClientReturnType = any;

export type GetPublicClientErrorType = Error;

export function getPublicClient<config extends Config>(
  config: config,
  parameters: GetPublicClientParameters = {},
): GetPublicClientReturnType {
  return config.getClient(parameters).extend(publicActions);
}
