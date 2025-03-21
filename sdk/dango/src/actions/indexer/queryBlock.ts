import { gql } from "graphql-request";
import { queryIndexer } from "./queryIndexer.js";

import type { Transport } from "@left-curve/sdk/types";
import type { DangoClient } from "../../types/clients.js";
import type { IndexedBlock } from "../../types/indexer.js";

export type QueryBlockParameters = {
  height: number;
};

export type QueryBlockReturnType = Promise<IndexedBlock>;

export async function queryBlock<transport extends Transport>(
  client: DangoClient<transport>,
  parameters: QueryBlockParameters,
): QueryBlockReturnType {
  const document = gql`
    query block($height: Int){
      block(height: $height) {
        createdAt,
        hash,
        transactions {
          hash
          sender
          transactionType
          hasSucceeded
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
