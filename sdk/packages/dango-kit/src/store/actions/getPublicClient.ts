import type { Client, Transport } from "@left-curve/types";
import { publicActions } from "../../actions/publicActions.js";
import type { PublicClient } from "../../clients/publicClient.js";
import type { Chain, Config, Signer } from "../../types/index.js";

export type GetPublicClientParameters = {
  chainId?: string;
};

export type GetPublicClientReturnType = PublicClient;

export type GetPublicClientErrorType = Error;

export function getPublicClient<config extends Config>(
  config: config,
  parameters: GetPublicClientParameters = {},
): GetPublicClientReturnType {
  const client = config.getClient(parameters) as unknown as Client<Transport, Chain, Signer>;
  return client.extend(publicActions) as unknown as PublicClient;
}
