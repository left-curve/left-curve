import type { Chain, ChainStatusResponse, Client, Signer, Transport } from "../types/index.js";

export type GetChainInfoReturnType = Promise<ChainStatusResponse>;

/**
 * Get the chain information.
 * @param parameters
 * @returns The chain information.
 */
export async function getChainInfo<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(client: Client<Transport, chain, signer>): GetChainInfoReturnType {
  return await client.request({ method: "query_status" });
}
