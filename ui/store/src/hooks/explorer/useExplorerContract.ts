import { useQuery } from "@tanstack/react-query";
import { useAppConfig } from "../useAppConfig.js";
import { usePublicClient } from "../usePublicClient.js";

import type { Address, Coins, ContractInfo } from "@left-curve/dango/types";

export type ExplorerContract = (ContractInfo & { address: Address; balances: Coins }) | null;

export function useExplorerContract(address: Address) {
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  return useQuery<ExplorerContract>({
    queryKey: ["contract_explorer", address],
    queryFn: async () => {
      const [contractInfo, balances] = await Promise.all([
        client.getContractInfo({ address }),
        client.getBalances({ address }),
      ]);

      const isAccount = appConfig.accountFactory.codeHash === contractInfo.codeHash;

      if (isAccount) return null;

      return {
        ...contractInfo,
        address,
        balances,
      };
    },
    enabled: !!address,
  });
}
