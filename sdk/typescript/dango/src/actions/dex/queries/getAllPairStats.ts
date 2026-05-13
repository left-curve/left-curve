import { queryIndexer } from "#actions/indexer/queryIndexer.js";

import type { Client, PairStats } from "@left-curve/types";

export type GetAllPairStatsReturnType = Promise<PairStats[]>;

export async function getAllPairStats(client: Client): GetAllPairStatsReturnType {
  const document = /* GraphQL */ `
    query AllPairStats {
      allPairStats {
        baseDenom
        quoteDenom
        currentPrice
        price24HAgo
        volume24H
        priceChange24H
      }
    }
  `;

  const { allPairStats } = await queryIndexer<{ allPairStats: PairStats[] }>(client, {
    document,
  });

  return allPairStats;
}
