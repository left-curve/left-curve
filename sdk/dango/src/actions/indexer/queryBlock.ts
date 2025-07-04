import { queryIndexer } from "./queryIndexer.js";

import type { Client, Transport } from "@left-curve/sdk/types";
import type { IndexedBlock } from "#types/indexer.js";

export type QueryBlockParameters = {
  height?: number;
};

export type QueryBlockReturnType = Promise<IndexedBlock>;

export async function queryBlock<transport extends Transport>(
  client: Client<transport>,
  parameters: QueryBlockParameters = {},
): QueryBlockReturnType {
  const document = /* GraphQL */ `
    query block($height: Int) {
      block(height: $height) {
        createdAt
        hash
        blockHeight
        appHash
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
