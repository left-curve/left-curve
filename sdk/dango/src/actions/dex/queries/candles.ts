import { queryIndexer } from "#actions/indexer/queryIndexer.js";

import type { Client, DateTime, Transport } from "@left-curve/sdk/types";
import type { Candle, CandleIntervals } from "#types/dex.js";
import type { GraphqlQueryResult } from "#types/graphql.js";

export type QueryCandlesParameters = {
  after?: string;
  first?: number;
  baseDenom: string;
  quoteDenom: string;
  interval: CandleIntervals;
  earlierThan?: DateTime;
  laterThan?: DateTime;
};

export type QueryCandlesReturnType = Promise<GraphqlQueryResult<Candle>>;

export async function queryCandles<transport extends Transport>(
  client: Client<transport>,
  parameters: QueryCandlesParameters,
): QueryCandlesReturnType {
  const document = /* GraphQL */ `
    query candles(
      $after: String
      $first: Int
      $baseDenom: String!
      $quoteDenom: String!
      $interval: CandleInterval!
      $earlierThan: DateTime
      $laterThan: DateTime
    ) {
      candles(
        after: $after
        first: $first
        baseDenom: $baseDenom
        quoteDenom: $quoteDenom
        interval: $interval
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
          quoteDenom
          baseDenom
          interval
          blockHeight
          open
          high
          low
          close
          volumeBase
          volumeQuote
          timeStart
          timeStartUnix
          timeEnd
          timeEndUnix
        }
      }
    }
  `;

  const { candles } = await queryIndexer<{ candles: GraphqlQueryResult<Candle> }>(client, {
    document,
    variables: parameters,
  });

  return candles;
}
