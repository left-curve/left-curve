import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client } from "../../../types/index.js";
import type { PairStats } from "../../../types/dex.js";

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
