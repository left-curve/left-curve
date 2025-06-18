import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useAccount } from "./useAccount.js";
import { usePublicClient } from "./usePublicClient.js";

import type { PairId } from "@left-curve/dango/types";

export type UseProTradeParameters = {
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
  inputs: Record<string, { value: string }>;
};

export function useProTrade(parameters: UseProTradeParameters) {
  const { inputs, pairId, onChangePairId } = parameters;
  const { account } = useAccount();
  const publicClient = usePublicClient();

  const [operation, setOperation] = useState<"market" | "limit">("limit");
  const [action, setAction] = useState<"buy" | "sell">("buy");

  const orders = useQuery({
    enabled: !!account,
    queryKey: ["ordersByUser", account?.address],
    queryFn: async () => {
      if (!account) return [];
      const response = await publicClient.ordersByUser({ user: account.address });
      return Object.entries(response).map(([id, order]) => ({
        ...order,
        id: +id,
      }));
    },
    initialData: [],
    refetchInterval: 1000 * 10,
  });

  return {
    pairId,
    onChangePairId,
    orders,
    operation,
    setOperation,
    action,
    setAction,
    type: "spot",
  };
}
