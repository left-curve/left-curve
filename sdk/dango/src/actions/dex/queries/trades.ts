import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Address, Client, Transport } from "@left-curve/sdk/types";
import type { Trade } from "../../../types/dex.js";
import type { GraphqlQueryResult } from "../../../types/graphql.js";

export type QueryTradesParameters = {
  after?: string;
  first?: number;
  address?: Address;
};

export type QueryTradesReturnType = Promise<GraphqlQueryResult<Trade>>;

export async function queryTrades<transport extends Transport>(
  client: Client<transport>,
  parameters: QueryTradesParameters,
): QueryTradesReturnType {
  const document = /* GraphQL */ `
    query trades($after: String, $first: Int, $address: String) {
      trades(after: $after, first: $first, addr: $address) {
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
          orderType
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
