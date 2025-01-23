import { createBaseClient } from "@left-curve/sdk";
import { publicActions, signerActions } from "../actions/index.js";

import type { Transport } from "@left-curve/types";

import type { Chain } from "../types/chain.js";
import type { SignerClient, SignerClientConfig } from "../types/clients.js";
import type { Signer } from "../types/signer.js";

export function createSignerClient<transport extends Transport = Transport>(
  parameters: SignerClientConfig<transport>,
): SignerClient<transport> {
  const { name = "Dango Signer Client", type = "dango", username } = parameters;

  const client = createBaseClient<transport, Chain, Signer, { username: string }>({
    ...parameters,
    name,
    type,
    username,
  });

  return client.extend(publicActions).extend(signerActions);
}
