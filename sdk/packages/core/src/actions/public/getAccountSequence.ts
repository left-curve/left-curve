import type { Address, Chain, Client, Signer, Transport } from "@leftcurve/types";
import { queryWasmSmart } from "./queryWasmSmart.js";

export type GetAccountSequenceParameters = {
  address: Address;
  height?: number;
};

export type GetAccountSequenceReturnType = Promise<number>;

/**
 * Get the account state.
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.height The height at which to query the account state.
 * @returns The account state.
 */
export async function getAccountSequence<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountSequenceParameters,
): GetAccountSequenceReturnType {
  const { address, height = 0 } = parameters;
  const msg = { sequence: {} };
  return await queryWasmSmart<number, chain, signer>(client, {
    contract: address,
    msg,
    height,
  });
}
