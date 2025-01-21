import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";
import type { Chain, Client, Signer, Transport } from "@left-curve/types";
import type { AppConfig, Key, KeyHash } from "../../../types/index.js";

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

  const { addresses } = await getAppConfig<AppConfig>(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
