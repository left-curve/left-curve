import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client } from "../../../types/index.js";
import type { PerpsPairStats } from "../../../types/dex.js";

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
