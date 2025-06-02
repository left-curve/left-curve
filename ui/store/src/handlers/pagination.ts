import type { GraphqlQueryResult } from "@left-curve/dango/types";

type WithPaginationParameters = {
  limit?: number;
};

export function withPagination<T = unknown>(parameters: WithPaginationParameters) {
  const { limit = 10 } = parameters;

  function getNextPageParam({ pageInfo }: GraphqlQueryResult<T>) {
    if (!pageInfo.hasNextPage) return undefined;

    return {
      first: limit,
      after: pageInfo.endCursor as string,
    };
  }

  function getPreviousPageParam({ pageInfo }: GraphqlQueryResult<T>) {
    if (!pageInfo.hasPreviousPage) return undefined;

    return {
      first: limit,
      before: pageInfo.startCursor as string,
    };
  }

  const initialPageParam = {
    first: limit,
  };

  return {
    initialPageParam,
    getNextPageParam,
    getPreviousPageParam,
  };
}
