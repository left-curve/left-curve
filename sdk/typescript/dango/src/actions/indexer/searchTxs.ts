import type {
  Client,
  GraphqlPagination,
  GraphqlQueryResult,
  IndexedTransaction,
  Prettify,
} from "@left-curve/types";
import { queryIndexer } from "./queryIndexer.js";

export type SearchTxsParameters = Prettify<
  GraphqlPagination & {
    hash?: string;
    senderAddress?: string;
  }
>;

export type SearchTxsReturnType = Promise<GraphqlQueryResult<IndexedTransaction>>;

export async function searchTxs(
  client: Client,
  parameters: SearchTxsParameters,
): SearchTxsReturnType {
  const document = /* GraphQL */ `
    query tx(
      $hash: String
      $senderAddress: String
      $after: String
      $before: String
      $first: Int
      $last: Int
      $sortBy: String
    ) {
      transactions(
        hash: $hash
        senderAddress: $senderAddress
        after: $after
        before: $before
        first: $first
        last: $last
        sortBy: $sortBy
      ) {
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
            orderIdx
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
