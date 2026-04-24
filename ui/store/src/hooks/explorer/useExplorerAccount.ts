import { useQuery } from "@tanstack/react-query";
import { usePublicClient } from "../usePublicClient.js";

import type {
  AccountDetails,
  Address,
  Coins,
  ContractInfo,
  PerpsOrdersByUserResponse,
  PerpsUserStateExtended,
  PerpsVaultState,
} from "@left-curve/dango/types";

export type ExplorerPerpsData = {
  userState: PerpsUserStateExtended | null;
  orders: PerpsOrdersByUserResponse | null;
  vaultState: PerpsVaultState | null;
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
      const [account, contractInfo, balances, perpsUserState, perpsOrders, perpsVaultState] =
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
          client
            .getPerpsVaultState()
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
          vaultState: perpsVaultState,
        },
      };
    },
    enabled: !!address,
  });
}
