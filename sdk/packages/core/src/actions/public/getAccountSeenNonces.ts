import type { Address, Chain, Client, Signer, Transport } from "@left-curve/types";
import { queryWasmSmart } from "./queryWasmSmart.js";

export type GetAccountNonceParameters = {
  address: Address;
  height?: number;
};

export type GetAccountNonceReturnType = Promise<[number, number[]]>;

/**
 * Get the most recent nonces that have been used to send transactions.
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.height The height at which to query the account state.
 * @returns An array of nonces.
 */
export async function getAccountSeenNonces<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountNonceParameters,
): GetAccountNonceReturnType {
  const { address, height = 0 } = parameters;
  const msg = { seenNonces: {} };
  const nonces = await queryWasmSmart<number[], chain, signer>(client, {
    contract: address,
    msg,
    height,
  });

  const currentNonce = nonces.length ? nonces[nonces.length - 1] + 1 : 0;
  return [currentNonce, nonces];
}
