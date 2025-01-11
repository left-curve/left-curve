import type { Chain, ChainInfoResponse, Client, Signer, Transport } from "@left-curve/types";

export type GetChainInfoReturnType = Promise<ChainInfoResponse>;

/**
 * Get the chain information.
 * @param parameters
 * @returns The chain information.
 */
export async function getChainInfo<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(client: Client<Transport, chain, signer>): GetChainInfoReturnType {
  const { block } = await client.request({
    method: "block",
    params: {},
  });

  return {
    chainId: block.header.chain_id,
    lastFinalizedBlock: {
      hash: block.header.last_block_id.hash,
      height: block.header.height,
      timestamp: block.header.time,
    },
  };
}
