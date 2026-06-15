import { queryIndexer } from "./queryIndexer.js";

import type { Client, IndexedBlock } from "@left-curve/types";

export type QueryBlockParameters = {
  height?: number;
};

export type QueryBlockReturnType = Promise<IndexedBlock>;

export async function queryBlock(
  client: Client,
  parameters: QueryBlockParameters = {},
): QueryBlockReturnType {
  const document = /* GraphQL */ `
    query block($height: Int) {
      block(height: $height) {
        createdAt
        hash
        blockHeight
        appHash
        cronsOutcomes
        transactions {
          hash
          sender
          blockHeight
          createdAt
          transactionType
          hasSucceeded
          messages {
            methodName
            contractAddr
          }
        }
      }
    }
  `;

  const { block } = await queryIndexer<{ block: IndexedBlock }>(client, {
    document,
    variables: parameters,
  });

  return block;
}
