import { queryIndexer } from "#actions/indexer/queryIndexer.js";

import type { Client, PerpsPairStats } from "@left-curve/types";

export type GetPerpsPairStatsParameters = {
  pairId: string;
};

export type GetPerpsPairStatsReturnType = Promise<PerpsPairStats>;

export async function getPerpsPairStats(
  client: Client,
  parameters: GetPerpsPairStatsParameters,
): GetPerpsPairStatsReturnType {
  const document = /* GraphQL */ `
    query PerpsPairStats($pairId: String!) {
      perpsPairStats(pairId: $pairId) {
        pairId
        currentPrice
        price24HAgo
        volume24H
        priceChange24H
      }
    }
  `;

  const { perpsPairStats } = await queryIndexer<{ perpsPairStats: PerpsPairStats }>(client, {
    document,
    variables: parameters,
  });

  return perpsPairStats;
}
