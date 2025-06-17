import { useQuery } from "@tanstack/react-query";
import { useAccount } from "./useAccount.js";
import { usePublicClient } from "./usePublicClient.js";

export type UseProTradeParameters = {};

export function useProTrade(parameters: UseProTradeParameters) {
  const { account } = useAccount();
  const publicClient = usePublicClient();

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
    orders,
  };
}
