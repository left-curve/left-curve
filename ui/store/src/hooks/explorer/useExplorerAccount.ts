import { useQuery } from "@tanstack/react-query";
import { usePublicClient } from "../usePublicClient.js";

import type { Account, Address, Coins, ContractInfo } from "@left-curve/dango/types";

export type ExplorerAccount = (Account & ContractInfo & { balances: Coins }) | null;

export function useExplorerAccount(address: Address) {
  const client = usePublicClient();

  return useQuery<ExplorerAccount>({
    queryKey: ["account_explorer", address],
    queryFn: async () => {
      const [account, contractInfo, balances] = await Promise.all([
        client.getAccountInfo({ address }),
        client.getContractInfo({ address }),
        client.getBalances({ address }),
      ]);

      if (!account) return null;

      return {
        ...account,
        ...contractInfo,
        balances,
      };
    },
    enabled: !!address,
  });
}
