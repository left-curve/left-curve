import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Address, Base64, Client } from "@left-curve/types";
import { getAccountSeenNonces } from "./getAccountSeenNonces.js";

export type GetAccountSessionSeenNoncesParameters = {
  address: Address;
  sessionKey: Base64;
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
 */
export async function getAccountSessionSeenNonces(
  client: Client,
  parameters: GetAccountSessionSeenNoncesParameters,
): GetAccountSessionSeenNoncesReturnType {
  const { address, sessionKey } = parameters;
  const msg = { sessionSeenNonces: { sessionKey } };
  const nonces = await queryWasmSmart<number[]>(client, {
    contract: address,
    msg,
  });

  if (nonces.length > 0) {
    return [nonces[nonces.length - 1] + 1, nonces];
  }

  const [standardNext] = await getAccountSeenNonces(client, { address });
  return [standardNext, nonces];
}
