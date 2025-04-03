import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Signer, Transport } from "@left-curve/sdk/types";
import type { AppConfig, Key, KeyHash } from "../../../types/index.js";

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

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
