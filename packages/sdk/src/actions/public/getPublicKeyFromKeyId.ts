import type { Address, Chain, Client, Key, KeyId, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetPublicKeyFromKeyIdParameters = {
  keyId: KeyId;
  height?: number;
};

export type GetPublicKeyFromKeyIdReturnType = Promise<Key>;

/**
 * Get the public key from a key id.
 * @param parameters
 * @param parameters.keyId The key id to get the public key for.
 * @param parameters.height The height at which to get the public key.
 * @returns The public key.
 */
export async function getPublicKeyFromKeyId<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetPublicKeyFromKeyIdParameters,
): GetPublicKeyFromKeyIdReturnType {
  const { keyId, height = 0 } = parameters;
  const msg = { key: keyId };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
