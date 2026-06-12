import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { sharesToUsd } from "@left-curve/utils";

import { usePublicClient } from "./usePublicClient.js";
import { usePerpsUserState } from "./usePerpsUserState.js";

export type UsePerpsVaultUserSharesParameters = {
  accountAddress?: string;
  enabled?: boolean;
};

export function usePerpsVaultUserShares(parameters: UsePerpsVaultUserSharesParameters) {
  const { accountAddress, enabled = true } = parameters;
  const publicClient = usePublicClient();
  const userVaultShares = usePerpsUserState((state) => state.userState?.vaultShares ?? "0", {
    accountAddress,
    enabled,
  });

  const { data: vaultState } = useQuery({
    queryKey: ["vaultState"],
    queryFn: async () => publicClient.getPerpsVaultState(),
    enabled,
    refetchInterval: 10_000,
  });

  const userSharesValue = useMemo(() => {
    const shareSupply = vaultState?.shareSupply ?? "0";
    const equity = vaultState?.equity ?? "0";
    return sharesToUsd(userVaultShares, equity, shareSupply);
  }, [userVaultShares, vaultState]);

  return {
    vaultState,
    userVaultShares,
    userSharesValue,
  };
}
