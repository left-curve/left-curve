import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import type { GraphqlPagination, GraphqlQueryResult } from "@left-curve/dango/types";
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

  const [pagination, setPagination] = useState<GraphqlPagination>({
    first: limit,
    last: undefined,
    after: undefined,
    before: undefined,
    sortBy,
  });

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
        first: 10,
        after: data.pageInfo.endCursor as string,
        sortBy: "BLOCK_HEIGHT_DESC",
      });
    }
  };

  const goPrev = () => {
    if (data?.pageInfo?.hasPreviousPage) {
      setPagination({
        last: 10,
        before: data.pageInfo.startCursor as string,
        sortBy: "BLOCK_HEIGHT_DESC",
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
