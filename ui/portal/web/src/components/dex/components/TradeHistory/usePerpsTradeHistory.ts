import { useAccount, useInfiniteGraphqlQuery, usePublicClient } from "@left-curve/store";
import type { PerpsEvent } from "@left-curve/types";
import { useCallback, useMemo } from "react";

import type { QueryRange } from "./tradeHistoryFilterContext";

const PAGE_SIZE = 30;

export type UsePerpsTradeHistoryResult = {
  nodes: PerpsEvent[];
  isLoading: boolean;
  isFetchingNextPage: boolean;
  hasNextPage: boolean;
  fetchNextPage: () => void;
};

export function usePerpsTradeHistory(queryRange: QueryRange): UsePerpsTradeHistoryResult {
  const { account } = useAccount();
  const publicClient = usePublicClient();
  const { earlierThan, laterThan } = queryRange;
  const address = account?.address;

  const infiniteQuery = useInfiniteGraphqlQuery<PerpsEvent>({
    limit: PAGE_SIZE,
    query: {
      enabled: !!address,
      queryKey: ["perpsTradeHistory", address ?? "", earlierThan ?? "", laterThan ?? ""],
      queryFn: async ({ pageParam }) => {
        if (!address) throw new Error("missing account");
        return await publicClient.queryPerpsEvents({
          userAddr: address,
          sortBy: "BLOCK_HEIGHT_DESC",
          earlierThan,
          laterThan,
          first: pageParam.first,
          last: pageParam.last,
          after: pageParam.after,
          before: pageParam.before,
        });
      },
    },
  });

  const nodes = useMemo(
    () => infiniteQuery.data?.pages.flatMap((page) => page.nodes) ?? [],
    [infiniteQuery.data],
  );

  const fetchNextPage = useCallback(() => {
    if (infiniteQuery.hasNextPage && !infiniteQuery.isFetchingNextPage) {
      infiniteQuery.fetchNextPage();
    }
  }, [infiniteQuery.fetchNextPage, infiniteQuery.hasNextPage, infiniteQuery.isFetchingNextPage]);

  return {
    nodes,
    isLoading: infiniteQuery.isLoading,
    isFetchingNextPage: infiniteQuery.isFetchingNextPage,
    hasNextPage: infiniteQuery.hasNextPage,
    fetchNextPage,
  };
}
