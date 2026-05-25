import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import type { GraphqlPagination, GraphqlQueryResult } from "@left-curve/types";
import type {
  DefaultError,
  QueryClient,
  QueryFunction,
  QueryFunctionContext,
  UseQueryOptions,
} from "@tanstack/react-query";

export type UseQueryWithPaginationParameters<
  TData = unknown,
  TError = DefaultError,
  TQueryKey extends string[] = string[],
> = {
  limit?: number;
  sortBy?: string;
  queryFn: QueryFunction<GraphqlQueryResult<TData>, TQueryKey, GraphqlPagination>;
} & Omit<
  UseQueryOptions<GraphqlQueryResult<TData>, TError, GraphqlQueryResult<TData>, TQueryKey>,
  "queryFn"
>;

export function useQueryWithPagination<
  TData = unknown,
  TError = DefaultError,
  TQueryKey extends string[] = string[],
>(
  parameters: UseQueryWithPaginationParameters<TData, TError, TQueryKey>,
  queryClient?: QueryClient,
) {
  const { limit = 10, sortBy = "BLOCK_HEIGHT_DESC", queryFn, ...query } = parameters;

  const initialPagination: GraphqlPagination = {
    first: limit,
    last: undefined,
    after: undefined,
    before: undefined,
    sortBy,
  };

  const [pagination, setPagination] = useState<GraphqlPagination>(initialPagination);

  // Reset to the first page whenever the caller-provided queryKey changes
  // (e.g. when a filter like a date range is updated). Without this, the cursor
  // from the previous query would stay applied and the user would land on an
  // arbitrary page of the new dataset.
  const serializedQueryKey = JSON.stringify(query.queryKey);
  useEffect(() => {
    setPagination({
      first: limit,
      last: undefined,
      after: undefined,
      before: undefined,
      sortBy,
    });
  }, [serializedQueryKey, limit, sortBy]);

  const { data, ...response } = useQuery<
    GraphqlQueryResult<TData>,
    TError,
    GraphqlQueryResult<TData>,
    TQueryKey
  >(
    {
      ...query,
      queryFn: (params) =>
        queryFn({ ...params, pageParam: pagination } as QueryFunctionContext<
          TQueryKey,
          GraphqlPagination
        >),
      queryKey: [...query.queryKey, pagination.sortBy, pagination.after, pagination.before].filter(
        (v) => typeof v === "string",
      ) as TQueryKey,
      placeholderData: keepPreviousData,
    },
    queryClient,
  );

  const goNext = () => {
    if (data?.pageInfo?.hasNextPage) {
      setPagination({
        before: undefined,
        last: undefined,
        first: limit,
        after: data.pageInfo.endCursor as string,
        sortBy,
      });
    }
  };

  const goPrev = () => {
    if (data?.pageInfo?.hasPreviousPage) {
      setPagination({
        after: undefined,
        first: undefined,
        last: limit,
        before: data.pageInfo.startCursor as string,
        sortBy,
      });
    }
  };

  const hasNextPage = data?.pageInfo?.hasNextPage ?? false;

  const hasPreviousPage = data?.pageInfo?.hasPreviousPage ?? false;

  return {
    data,
    ...response,
    pagination: {
      goNext,
      goPrev,
      hasNextPage,
      hasPreviousPage,
    },
  };
}
