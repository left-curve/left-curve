import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { sharesToUsd } from "@left-curve/dango/utils";

import { usePublicClient } from "./usePublicClient.js";
import { perpsUserStateStore } from "./usePerpsUserState.js";

export function usePerpsVaultUserShares() {
  const publicClient = usePublicClient();
  const perpsUserState = perpsUserStateStore((s) => s.userState);

  const { data: vaultState } = useQuery({
    queryKey: ["vaultState"],
    queryFn: async () => publicClient.getPerpsVaultState(),
    refetchInterval: 10_000,
  });

  const userVaultShares = perpsUserState?.vaultShares ?? "0";

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
