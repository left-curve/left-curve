import { publicActions } from "@left-curve/dango";

import type { Chain, PublicClient, Signer } from "@left-curve/dango/types";
import type { Client, Transport } from "@left-curve/dango/types";

import type { Config } from "../types/index.js";

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
