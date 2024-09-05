import type { Address, Chain, Client, Key, KeyHash, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetKeyParameters = {
  username: string;
  keyHash: KeyHash;
  height?: number;
};

export type GetKeyReturnType = Promise<Key>;

/**
 * Given a key hash and username get a public key.
 * @param parameters
 * @param parameters.username The username of the account.
 * @param parameters.keyHash The key hash of the key.
 * @param parameters.height The height at which to get the public key.
 * @returns The public key.
 */
export async function getKey<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetKeyParameters,
): GetKeyReturnType {
  const { username, keyHash, height = 0 } = parameters;
  const msg = { key: { username, keyHash } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
