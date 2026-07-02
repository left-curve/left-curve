import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Address, Client } from "@left-curve/types";

export type GetAccountSeenNoncesParameters = {
  address: Address;
};

export type GetAccountSeenNoncesReturnType = Promise<[number, number[]]>;

/**
 * Get the most recent nonces that have been used to send transactions.
 * @param parameters
 * @param parameters.address The address of the account.
 * @returns An array of nonces.
 */
export async function getAccountSeenNonces(
  client: Client,
  parameters: GetAccountSeenNoncesParameters,
): GetAccountSeenNoncesReturnType {
  const { address } = parameters;
  const msg = { seenNonces: {} };
  const nonces = await queryWasmSmart<number[]>(client, {
    contract: address,
    msg,
  });

  const currentNonce = nonces.length ? nonces[nonces.length - 1] + 1 : 0;
  return [currentNonce, nonces];
}
