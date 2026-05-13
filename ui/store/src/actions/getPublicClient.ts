import { publicActions } from "@left-curve/sdk";

import type { PublicClient } from "@left-curve/sdk";

import type { Config } from "../types/index.js";

export type GetPublicClientReturnType = PublicClient;

export type GetPublicClientErrorType = Error;

export function getPublicClient<config extends Config>(config: config): GetPublicClientReturnType {
  const client = config.getClient();
  return client.extend(publicActions) as unknown as PublicClient;
}
