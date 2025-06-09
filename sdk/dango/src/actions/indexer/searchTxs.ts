import type { Client, Prettify, Transport } from "@left-curve/sdk/types";
import { queryIndexer } from "./queryIndexer.js";

import type { GraphqlPagination, GraphqlQueryResult } from "#types/graphql.js";
import type { IndexedTransaction } from "#types/indexer.js";

export type SearchTxsParameters = Prettify<
  GraphqlPagination & {
    hash?: string;
    senderAddress?: string;
  }
>;

export type SearchTxsReturnType = Promise<GraphqlQueryResult<IndexedTransaction>>;

export async function searchTxs<transport extends Transport>(
  client: Client<transport>,
  parameters: SearchTxsParameters,
): SearchTxsReturnType {
  const document = `
    query tx($hash: String, $senderAddress: String, $after: String, $before: String, $first: Int, $last: Int, $sortBy: String) {
      transactions(hash: $hash, senderAddress: $senderAddress, after: $after, before: $before, first: $first, last: $last, sortBy: $sortBy) {
        pageInfo {
          hasNextPage
          hasPreviousPage
          startCursor
          endCursor
        }
        nodes {
          hash
          blockHeight
          transactionIdx
          hasSucceeded
          sender
          nestedEvents
          transactionType
          createdAt
          gasWanted
          gasUsed
          errorMessage
          messages {
            data
            methodName
            contractAddr
          }
        }
      }
    }
  `;

  type TxReturnType = {
    transactions: GraphqlQueryResult<IndexedTransaction>;
  };

  const { transactions } = await queryIndexer<TxReturnType>(client, {
    document,
    variables: parameters,
  });

  return transactions;
}
