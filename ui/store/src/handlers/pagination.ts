import type { GraphqlQueryResult } from "@left-curve/dango/types";

type WithPaginationParameters = {
  limit?: number;
  sortBy?: string;
};

export function withPagination<T = unknown>(parameters: WithPaginationParameters) {
  const { limit = 10, sortBy } = parameters;

  function getNextPageParam({ pageInfo }: GraphqlQueryResult<T>) {
    if (!pageInfo.hasNextPage) return undefined;

    return {
      first: limit,
      after: pageInfo.endCursor as string,
      sortBy,
    };
  }

  function getPreviousPageParam({ pageInfo }: GraphqlQueryResult<T>) {
    if (!pageInfo.hasPreviousPage) return undefined;

    return {
      last: limit,
      before: pageInfo.startCursor as string,
      sortBy,
    };
  }

  const initialPageParam = {
    first: limit,
    sortBy,
  };

  return {
    initialPageParam,
    getNextPageParam,
    getPreviousPageParam,
  };
}
