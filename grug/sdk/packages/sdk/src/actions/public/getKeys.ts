import type { Address, Chain, Client, Key, KeyHash, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetKeysParameters = {
  startAfter?: KeyHash;
  limit?: number;
  height?: number;
};

export type GetKeysReturnType = Promise<Record<KeyHash, Key>>;

/**
 * Get all keys in the factory.
 * @param parameters
 * @param parameters.startAfter The key hash to start after.
 * @param parameters.limit The maximum number of keys to return.
 * @param parameters.height The height at which to get the keys.
 * @returns A record of key hash and key.
 */
export async function getKeys<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetKeysParameters,
): GetKeysReturnType {
  const { startAfter, limit, height = 0 } = parameters;
  const msg = { keys: { startAfter, limit } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
