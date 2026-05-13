import { createBaseClient } from "./baseClient.js";
import { publicActions } from "#actions/publicActions.js";
import { signerActions } from "#actions/signerActions.js";

import type { Client, Signer, SignerClientConfig } from "@left-curve/types";
import type { PublicActions } from "#actions/publicActions.js";
import type { SignerActions } from "#actions/signerActions.js";

export type SignerClient = Client<Signer, PublicActions & SignerActions>;

export function createSignerClient(parameters: SignerClientConfig): SignerClient {
  const { name = "Signer Client", type = "dango" } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type,
  }) as unknown as Client;

  const publicClient = client.extend(publicActions) as unknown as SignerClient;
  return publicClient.extend(signerActions) as SignerClient;
}
