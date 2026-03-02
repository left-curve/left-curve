import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client, Transport } from "@left-curve/sdk/types";
import type { PairStats } from "../../../types/dex.js";

export type GetPairStatsParameters = {
  baseDenom: string;
  quoteDenom: string;
};

export type GetPairStatsReturnType = Promise<PairStats>;

export async function getPairStats<transport extends Transport>(
  client: Client<transport>,
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
