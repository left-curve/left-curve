import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client } from "../../../types/index.js";
import type { PairStats } from "../../../types/dex.js";

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
