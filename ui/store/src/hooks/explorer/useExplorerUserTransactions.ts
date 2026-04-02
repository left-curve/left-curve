import { useQueries } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { usePublicClient } from "../usePublicClient.js";

import type { Address, IndexedTransaction } from "@left-curve/dango/types";

const PAGE_SIZE = 10;

export function useExplorerUserTransactions(addresses: Address[]) {
  const client = usePublicClient();
  const [page, setPage] = useState(0);

  const transactionQueries = useQueries({
    queries: addresses.map((address) => ({
      queryKey: ["explorer_user_transactions", address],
      queryFn: async () => {
        const result = await client.searchTxs({
          senderAddress: address,
          first: 50,
          sortBy: "BLOCK_HEIGHT_DESC",
        });
        return result.nodes;
      },
      enabled: addresses.length > 0,
    })),
  });

  const isLoading = transactionQueries.some((q) => q.isLoading);

  const allTransactions = useMemo(() => {
    const txs = transactionQueries
      .filter((q) => q.data)
      .flatMap((q) => q.data as IndexedTransaction[]);

    return txs.sort(
      (a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
    );
  }, [transactionQueries]);

  const totalPages = Math.ceil(allTransactions.length / PAGE_SIZE);
  const paginatedTransactions = allTransactions.slice(
    page * PAGE_SIZE,
    (page + 1) * PAGE_SIZE
  );

  const pagination = {
    isLoading,
    goNext: () => setPage((p) => Math.min(p + 1, totalPages - 1)),
    goPrev: () => setPage((p) => Math.max(p - 1, 0)),
    hasNextPage: page < totalPages - 1,
    hasPreviousPage: page > 0,
  };

  return {
    data: paginatedTransactions,
    allTransactions,
    isLoading,
    pagination,
  };
}
