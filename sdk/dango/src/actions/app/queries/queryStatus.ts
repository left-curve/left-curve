import type { BlockInfo, Client, Transport } from "@left-curve/sdk/types";
import { gql } from "graphql-request";
import type { Chain } from "../../../types/chain.js";
import type { Signer } from "../../../types/signer.js";
import { queryIndexer } from "../../indexer/queryIndexer.js";

export type QueryStatusReturnType = Promise<{
  chainId: string;
  block: BlockInfo;
}>;

/**
 * Get the chain information.
 * @param parameters
 * @returns The chain information.
 */
export async function queryStatus<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(client: Client<Transport, chain, signer>): QueryStatusReturnType {
  if (client.transport.type !== "http-graphql") {
    const { node_info, sync_info } = await client.request({ method: "status" });

    return {
      chainId: node_info.id,
      block: {
        height: sync_info.latest_block_height,
        timestamp: sync_info.latest_block_time,
        hash: sync_info.latest_block_hash,
      },
    };
  }

  const document = gql`
    query {
      queryStatus {
        chainId
        block {
          blockHeight
          timestamp
          hash
        }
      }
    }
  `;

  const { queryStatus: response } = await queryIndexer<
    { queryStatus: Awaited<QueryStatusReturnType> & { block: { blockHeight: number } } },
    chain,
    signer
  >(client, {
    document,
  });

  console.log(response);

  return {
    chainId: response.chainId,
    block: {
      ...response.block,
      height: response.block.blockHeight.toString(),
    },
  };
}
