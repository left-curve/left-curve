import type { Client, ClientConfig, RequiredBy, Transport } from "@left-curve/types";

import { createBaseClient, grugActions } from "@left-curve/sdk";
import type { Chain, Signer } from "../types/index.js";

export type SignerClientConfig<transport extends Transport = Transport> = RequiredBy<
  ClientConfig<transport, Chain, Signer>,
  "signer"
>;

export type SignerClient<transport extends Transport = Transport> = Client<
  transport,
  Chain,
  Signer,
  PublicActions<transport, Chain> & SignerActions<transport, Chain>
>;

export function createSignerClient<transport extends Transport = Transport>(
  parameters: SignerClientConfig<transport>,
): SignerClient<transport> {
  const { name = "Signer Client", type = "signer" } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type,
  })
    .extend(grugActions)
    .extend({});

  return client as SignerClient<transport>;
}
