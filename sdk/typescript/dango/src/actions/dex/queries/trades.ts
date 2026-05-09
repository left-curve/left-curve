import { queryIndexer } from "#actions/indexer/queryIndexer.js";

import type { Address, Client, GraphqlQueryResult, Trade } from "@left-curve/types";

export type QueryTradesParameters = {
  after?: string;
  first?: number;
  address?: Address;
  /** ISO 8601 timestamp — only return trades created at or before this date. */
  earlierThan?: string;
  /** ISO 8601 timestamp — only return trades created at or after this date. */
  laterThan?: string;
};

export type QueryTradesReturnType = Promise<GraphqlQueryResult<Trade>>;

export async function queryTrades(
  client: Client,
  parameters: QueryTradesParameters,
): QueryTradesReturnType {
  const document = /* GraphQL */ `
    query trades(
      $after: String
      $first: Int
      $address: String
      $earlierThan: DateTime
      $laterThan: DateTime
    ) {
      trades(
        after: $after
        first: $first
        addr: $address
        earlierThan: $earlierThan
        laterThan: $laterThan
      ) {
        pageInfo {
          hasNextPage
          hasPreviousPage
          startCursor
          endCursor
        }
        nodes {
          addr
          quoteDenom
          baseDenom
          timeInForce
          blockHeight
          direction
          createdAt
          filledBase
          filledQuote
          refundBase
          refundQuote
          feeBase
          feeQuote
          clearingPrice
        }
      }
    }
  `;

  const { trades } = await queryIndexer<{
    trades: GraphqlQueryResult<Trade>;
  }>(client, {
    document,
    variables: parameters,
  });

  return trades;
}
