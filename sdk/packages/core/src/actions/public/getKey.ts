import type { Address, Chain, Client, Key, KeyHash, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig.js";
import { queryWasmSmart } from "./queryWasmSmart.js";

export type GetKeyParameters = {
  keyHash: KeyHash;
  height?: number;
};

export type GetKeyReturnType = Promise<Key>;

/**
 * Given a key hash get a public key.
 * @param parameters
 * @param parameters.keyHash The key hash of the key.
 * @param parameters.height The height at which to get the public key.
 * @returns The public key.
 */
export async function getKey<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetKeyParameters,
): GetKeyReturnType {
  const { keyHash, height = 0 } = parameters;
  const msg = { key: { hash: keyHash } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
