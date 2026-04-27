import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client, Transport } from "@left-curve/sdk/types";
import type { PerpsEvent } from "../../../types/indexer.js";
import type { GraphqlQueryResult } from "../../../types/graphql.js";

export type QueryPerpsEventsParameters = {
  after?: string;
  before?: string;
  first?: number;
  last?: number;
  sortBy?: "BLOCK_HEIGHT_ASC" | "BLOCK_HEIGHT_DESC";
  userAddr?: string;
  eventType?: string;
  pairId?: string;
  blockHeight?: number;
};

export type QueryPerpsEventsReturnType = Promise<GraphqlQueryResult<PerpsEvent>>;

export async function queryPerpsEvents<transport extends Transport>(
  client: Client<transport>,
  parameters: QueryPerpsEventsParameters,
): QueryPerpsEventsReturnType {
  const document = /* GraphQL */ `
    query PerpsEvents(
      $after: String
      $before: String
      $first: Int
      $last: Int
      $sortBy: PerpsEventSortBy
      $userAddr: String
      $eventType: String
      $pairId: String
      $blockHeight: Int
    ) {
      perpsEvents(
        after: $after
        before: $before
        first: $first
        last: $last
        sortBy: $sortBy
        userAddr: $userAddr
        eventType: $eventType
        pairId: $pairId
        blockHeight: $blockHeight
      ) {
        pageInfo {
          hasNextPage
          hasPreviousPage
          startCursor
          endCursor
        }
        nodes {
          idx
          blockHeight
          txHash
          eventType
          userAddr
          pairId
          data
          createdAt
        }
      }
    }
  `;

  const { perpsEvents } = await queryIndexer<{
    perpsEvents: GraphqlQueryResult<PerpsEvent>;
  }>(client, {
    document,
    variables: parameters,
  });

  return perpsEvents;
}
