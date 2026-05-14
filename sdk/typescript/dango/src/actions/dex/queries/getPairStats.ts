import { queryIndexer } from "#actions/indexer/queryIndexer.js";

import type { Client, PairStats } from "@left-curve/types";

export type GetPairStatsParameters = {
  baseDenom: string;
  quoteDenom: string;
};

export type GetPairStatsReturnType = Promise<PairStats>;

export async function getPairStats(
  client: Client,
  parameters: GetPairStatsParameters,
): GetPairStatsReturnType {
  const document = /* GraphQL */ `
    query PairStats($baseDenom: String!, $quoteDenom: String!) {
      pairStats(baseDenom: $baseDenom, quoteDenom: $quoteDenom) {
        baseDenom
        quoteDenom
        currentPrice
        price24HAgo
        volume24H
        priceChange24H
      }
    }
  `;

  const { pairStats } = await queryIndexer<{ pairStats: PairStats }>(client, {
    document,
    variables: parameters,
  });

  return pairStats;
}
