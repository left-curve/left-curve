import { useQuery } from "@tanstack/react-query";
import { usePublicClient } from "../usePublicClient.js";

import type { IndexedTransaction } from "@left-curve/dango/types";

export type UseExplorerTransactionReturn = IndexedTransaction | null;

export function useExplorerTransaction(txHash: string) {
  const client = usePublicClient();

  return useQuery<UseExplorerTransactionReturn>({
    queryKey: ["tx", txHash],
    queryFn: async () => {
      const txs = await client.searchTxs({ hash: txHash });
      if (!txs.nodes.length) return null;
      return txs.nodes[0] || null;
    },
    enabled: !!txHash,
  });
}
