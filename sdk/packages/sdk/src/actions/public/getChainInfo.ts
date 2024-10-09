import type { Chain, ChainInfoResponse, Client, Signer, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetChainInfoParameters = {
  height?: number;
};

export type GetChainInfoReturnType = Promise<ChainInfoResponse>;

/**
 * Get the chain information.
 * @param parameters
 * @param parameters.height The height at which to query the chain information.
 * @returns The chain information.
 */
export async function getChainInfo<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetChainInfoParameters = {},
): GetChainInfoReturnType {
  const { height = 0 } = parameters;

  const [{ block }, response] = await Promise.all([
    client.request({
      method: "block",
      params: {},
    }),
    queryApp(client, { query: { config: {} }, height }),
  ]);

  if (!("config" in response)) {
    throw new Error(`expecting config response, got ${JSON.stringify(response)}`);
  }

  return {
    chainId: block.header.chain_id,
    config: response.config,
    lastFinalizedBlock: {
      hash: block.header.last_block_id.hash,
      height: block.header.height,
      timestamp: block.header.time,
    },
  };
}
