import { keepPreviousData, useInfiniteQuery } from "@tanstack/react-query";
import { useState } from "react";

import { withPagination } from "../handlers/pagination.js";

import type { GraphqlPagination, GraphqlQueryResult } from "@left-curve/dango/types";
import type { DefinedInitialDataInfiniteOptions, InfiniteData } from "@tanstack/react-query";

export type UseInfiniteGraphqlQueryParameters<T> = {
  limit?: number;
  sortBy?: string;
  initialPage?: number;
  query: Omit<
    DefinedInitialDataInfiniteOptions<
      GraphqlQueryResult<T>,
      Error,
      InfiniteData<GraphqlQueryResult<T>>,
      unknown[],
      GraphqlPagination
    >,
    "getNextPageParam" | "getPreviousPageParam" | "initialPageParam" | "initialData"
  >;
};

export function useInfiniteGraphqlQuery<T = unknown>(
  parameters: UseInfiniteGraphqlQueryParameters<T>,
) {
  const { limit = 10, initialPage = 1, sortBy, query: queryOptions } = parameters;
  const [currentPage, setCurrentPage] = useState(initialPage);

  const query = useInfiniteQuery<
    GraphqlQueryResult<T>,
    Error,
    InfiniteData<GraphqlQueryResult<T>>,
    unknown[],
    GraphqlPagination
  >({
    ...withPagination<T>({ limit, sortBy }),
    ...queryOptions,
    placeholderData: keepPreviousData,
  });

  const pagination = {
    goNext: () => {
      if (!query.hasNextPage) return;
      query.fetchNextPage();
      setCurrentPage((prev) => prev + 1);
    },
    goPrev: () => {
      if (!query.hasPreviousPage) return;
      query.fetchPreviousPage();
      setCurrentPage((prev) => prev - 1);
    },
    currentPage,
    hasNextPage: query.hasNextPage,
    hasPreviousPage: query.hasPreviousPage,
  };

  return { ...query, pagination, currentPage };
}
