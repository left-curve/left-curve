import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client } from "../../../types/index.js";
import type { PerpsPairStats } from "../../../types/dex.js";

export type GetAllPerpsPairStatsReturnType = Promise<PerpsPairStats[]>;

export async function getAllPerpsPairStats(client: Client): GetAllPerpsPairStatsReturnType {
  const document = /* GraphQL */ `
    query AllPerpsPairStats {
      allPerpsPairStats {
        pairId
        currentPrice
        price24HAgo
        volume24H
        priceChange24H
      }
    }
  `;

  const { allPerpsPairStats } = await queryIndexer<{ allPerpsPairStats: PerpsPairStats[] }>(
    client,
    {
      document,
    },
  );

  return allPerpsPairStats;
}
