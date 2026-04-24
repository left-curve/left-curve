import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client, DateTime, Transport } from "@left-curve/sdk/types";
import type { CandleIntervals, PerpsCandle } from "../../../types/dex.js";
import type { GraphqlQueryResult } from "../../../types/graphql.js";

export type QueryPerpsCandlesParameters = {
  after?: string;
  first?: number;
  pairId: string;
  interval: CandleIntervals;
  earlierThan?: DateTime;
  laterThan?: DateTime;
};

export type QueryPerpsCandlesReturnType = Promise<GraphqlQueryResult<PerpsCandle>>;

export async function queryPerpsCandles<transport extends Transport>(
  client: Client<transport>,
  parameters: QueryPerpsCandlesParameters,
): QueryPerpsCandlesReturnType {
  const document = /* GraphQL */ `
    query perpsCandles(
      $after: String
      $first: Int
      $pairId: String!
      $interval: CandleInterval!
      $earlierThan: DateTime
      $laterThan: DateTime
    ) {
      perpsCandles(
        after: $after
        first: $first
        pairId: $pairId
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
          pairId
          interval
          minBlockHeight
          maxBlockHeight
          open
          high
          low
          close
          volume
          volumeUsd
          timeStart
          timeStartUnix
          timeEnd
          timeEndUnix
        }
      }
    }
  `;

  const { perpsCandles } = await queryIndexer<{
    perpsCandles: GraphqlQueryResult<PerpsCandle>;
  }>(client, {
    document,
    variables: parameters,
  });

  return perpsCandles;
}
