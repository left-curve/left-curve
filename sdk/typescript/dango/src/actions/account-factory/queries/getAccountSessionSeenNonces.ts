import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Address, Base64, Client } from "@left-curve/types";
import { getAccountSeenNonces } from "./getAccountSeenNonces.js";

export type GetAccountSessionSeenNoncesParameters = {
  address: Address;
  sessionKey: Base64;
  height?: number;
};

export type GetAccountSessionSeenNoncesReturnType = Promise<[number, number[]]>;

/**
 * Get the most recent nonces used by a given session key on an account.
 *
 * If the session window is empty, falls back to the account's standard-nonce
 * high-water mark + 1 (or 0 if the account has never transacted). This matches
 * the on-chain floor: a session key's first nonce must exceed the standard
 * window's largest seen nonce.
 *
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.sessionKey Base64-encoded 33-byte compressed session pubkey.
 * @param parameters.height The height at which to query the account state.
 */
export async function getAccountSessionSeenNonces(
  client: Client,
  parameters: GetAccountSessionSeenNoncesParameters,
): GetAccountSessionSeenNoncesReturnType {
  const { address, sessionKey, height = 0 } = parameters;
  const msg = { sessionSeenNonces: { sessionKey } };
  const nonces = await queryWasmSmart<number[]>(client, {
    contract: address,
    msg,
    height,
  });

  if (nonces.length > 0) {
    return [nonces[nonces.length - 1] + 1, nonces];
  }

  const [standardNext] = await getAccountSeenNonces(client, { address, height });
  return [standardNext, nonces];
}
