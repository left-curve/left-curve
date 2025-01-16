import type { Client, ClientConfig, RequiredBy, Transport } from "@left-curve/types";

import { createBaseClient } from "@left-curve/sdk";
import { type PublicActions, publicActions } from "../actions/index.js";
import { signerActions } from "../actions/signerActions.js";
import type { Chain, Signer } from "../types/index.js";

export type SignerClientConfig<transport extends Transport = Transport> = RequiredBy<
  ClientConfig<transport, Chain, Signer>,
  "signer"
>;

export type SignerClient<transport extends Transport = Transport> = Client<
  transport,
  Chain,
  Signer,
  PublicActions<transport> & any
>;

export function createSignerClient<transport extends Transport = Transport>(
  parameters: SignerClientConfig<transport>,
): SignerClient<transport> {
  const { name = "Dango Signer Client", type = "dango" } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type,
  });

  return client.extend(publicActions).extend(signerActions);
}
