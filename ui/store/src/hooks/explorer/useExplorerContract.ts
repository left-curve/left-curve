import { useQuery } from "@tanstack/react-query";
import { useConfig } from "../useConfig.js";
import { usePublicClient } from "../usePublicClient.js";

import type { Address, Coins, ContractInfo } from "@left-curve/dango/types";

export type ExplorerContract = (ContractInfo & { address: Address; balances: Coins }) | null;

export function useExplorerContract(address: Address) {
  const client = usePublicClient();
  const { getAppConfig } = useConfig();

  return useQuery<ExplorerContract>({
    queryKey: ["contract_explorer", address],
    queryFn: async () => {
      const [appConfig, contractInfo, balances] = await Promise.all([
        getAppConfig(),
        client.getContractInfo({ address }),
        client.getBalances({ address }),
      ]);

      const isAccount = Object.values(appConfig.accountFactory.codeHashes).includes(
        contractInfo.codeHash,
      );

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
