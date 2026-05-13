import { queryWasmSmart } from "../../../index.js";
import type { Address, Client } from "../../../types/index.js";

export type GetAccountSeenNoncesParameters = {
  address: Address;
  height?: number;
};

export type GetAccountSeenNoncesReturnType = Promise<[number, number[]]>;

/**
 * Get the most recent nonces that have been used to send transactions.
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.height The height at which to query the account state.
 * @returns An array of nonces.
 */
export async function getAccountSeenNonces(
  client: Client,
  parameters: GetAccountSeenNoncesParameters,
): GetAccountSeenNoncesReturnType {
  const { address, height = 0 } = parameters;
  const msg = { seenNonces: {} };
  const nonces = await queryWasmSmart<number[]>(client, {
    contract: address,
    msg,
    height,
  });

  const currentNonce = nonces.length ? nonces[nonces.length - 1] + 1 : 0;
  return [currentNonce, nonces];
}
