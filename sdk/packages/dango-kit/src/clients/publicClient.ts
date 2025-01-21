import { createBaseClient } from "@left-curve/sdk";
import type { Client, ClientConfig, Transport } from "@left-curve/types";
import { type PublicActions, publicActions } from "../actions/index.js";
import type { Chain } from "../types/index.js";

export type PublicClientConfig<transport extends Transport = Transport> = ClientConfig<
  transport,
  Chain,
  undefined
>;

export type PublicClient<transport extends Transport = Transport> = Client<
  transport,
  Chain,
  undefined,
  PublicActions<transport>
>;

export function createPublicClient<transport extends Transport>(
  parameters: PublicClientConfig<transport>,
): PublicClient<transport> {
  const { name = "Dango Public Client" } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type: "dango",
  });

  return client.extend(publicActions as any);
}
