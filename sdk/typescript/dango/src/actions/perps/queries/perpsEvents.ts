import { queryIndexer } from "#actions/indexer/queryIndexer.js";

import type { Client, GraphqlQueryResult, PerpsEvent } from "@left-curve/types";

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
  /** ISO 8601 timestamp — only return events created at or before this date. */
  earlierThan?: string;
  /** ISO 8601 timestamp — only return events created at or after this date. */
  laterThan?: string;
};

export type QueryPerpsEventsReturnType = Promise<GraphqlQueryResult<PerpsEvent>>;

export async function queryPerpsEvents(
  client: Client,
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
      $earlierThan: DateTime
      $laterThan: DateTime
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
        earlierThan: $earlierThan
        laterThan: $laterThan
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
