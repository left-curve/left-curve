import { useQuery } from "@tanstack/react-query";
import { useAccount } from "./useAccount.js";
import { usePublicClient } from "./usePublicClient.js";

import type { OrdersByUserResponse, Prettify, WithId } from "@left-curve/dango/types";
import type { UseQueryOptions, UseQueryResult } from "@tanstack/react-query";

export type UseOrdersByUserParameters = Prettify<
  Omit<
    UseQueryOptions<
      WithId<OrdersByUserResponse>[],
      Error,
      WithId<OrdersByUserResponse>[],
      string[]
    >,
    "queryFn" | "queryKey"
  > & {
    queryKey?: string[];
  }
>;

export type UseOrdersByUserReturnType = UseQueryResult<WithId<OrdersByUserResponse>[], Error>;

export function useOrdersByUser(
  parameters: UseOrdersByUserParameters = {},
): UseOrdersByUserReturnType {
  const { account } = useAccount();
  const publicClient = usePublicClient();

  const { queryKey = [], enabled = account, ...rest } = parameters;

  const { address } = account || { address: "default" };

  return useQuery({
    enabled: !!enabled,
    queryKey: ["ordersByUser", address, ...queryKey],
    queryFn: async () => {
      if (!account) return [];
      const response = await publicClient.ordersByUser({ user: account.address });
      return Object.entries(response).map(([id, order]) => ({
        ...order,
        id,
      }));
    },
    ...rest,
  });
}
