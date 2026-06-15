import { queryIndexer } from "#actions/indexer/queryIndexer.js";

import type { BlockInfo, Client } from "@left-curve/types";

export type QueryStatusReturnType = Promise<{
  chainId: string;
  block: BlockInfo;
}>;

export async function queryStatus(client: Client): QueryStatusReturnType {
  const document = `
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

  const { queryStatus: response } = await queryIndexer<{
    queryStatus: Awaited<QueryStatusReturnType> & { block: { blockHeight: number } };
  }>(client, {
    document,
  });

  return {
    chainId: response.chainId,
    block: {
      ...response.block,
      height: response.block.blockHeight.toString(),
    },
  };
}
