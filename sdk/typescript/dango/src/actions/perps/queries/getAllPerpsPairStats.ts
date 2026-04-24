import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client, Transport } from "@left-curve/sdk/types";
import type { PerpsPairStats } from "../../../types/dex.js";

export type GetAllPerpsPairStatsReturnType = Promise<PerpsPairStats[]>;

export async function getAllPerpsPairStats<transport extends Transport>(
  client: Client<transport>,
): GetAllPerpsPairStatsReturnType {
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
