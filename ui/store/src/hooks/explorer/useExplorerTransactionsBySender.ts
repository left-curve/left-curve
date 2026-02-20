import { usePublicClient } from "../usePublicClient.js";
import { useQueryWithPagination } from "../useQueryWithPagination.js";

import type { Address } from "@left-curve/dango/types";

export function useExplorerTransactionsBySender(address: Address, enabled = true) {
  const client = usePublicClient();

  return useQueryWithPagination({
    enabled: enabled && !!address,
    queryKey: ["explorer_transactions", address],
    queryFn: async ({ pageParam }) => client.searchTxs({ senderAddress: address, ...pageParam }),
  });
}
