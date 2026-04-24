import { useQuery } from "@tanstack/react-query";
import { usePublicClient } from "../usePublicClient.js";

import type {
  AccountDetails,
  Address,
  Coins,
  ContractInfo,
  PerpsOrdersByUserResponse,
  PerpsUserStateExtended,
} from "@left-curve/dango/types";

export type ExplorerPerpsData = {
  userState: PerpsUserStateExtended | null;
  orders: PerpsOrdersByUserResponse | null;
};

export type ExplorerAccount =
  | (AccountDetails &
      ContractInfo & {
        balances: Coins;
        perps: ExplorerPerpsData;
      })
  | null;

export function useExplorerAccount(address: Address) {
  const client = usePublicClient();

  return useQuery<ExplorerAccount>({
    queryKey: ["account_explorer", address],
    queryFn: async () => {
      const [account, contractInfo, balances, perpsUserState, perpsOrders] =
        await Promise.all([
          client.getAccountInfo({ address }),
          client.getContractInfo({ address }),
          client.getBalances({ address }),
          client
            .getPerpsUserStateExtended({ user: address, includeAll: true })
            .catch(() => null),
          client
            .getPerpsOrdersByUser({ user: address })
            .catch(() => null),
        ]);

      if (!account) return null;

      return {
        ...account,
        ...contractInfo,
        balances,
        perps: {
          userState: perpsUserState,
          orders: perpsOrders,
        },
      };
    },
    enabled: !!address,
  });
}
