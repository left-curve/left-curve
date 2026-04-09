import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { Decimal } from "@left-curve/dango/utils";

import { usePublicClient } from "./usePublicClient.js";
import { perpsUserStateStore } from "./usePerpsUserState.js";

const VIRTUAL_SHARES = "1000000";
const VIRTUAL_ASSETS = "1";

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
    if (userVaultShares === "0") return "0";
    const effectiveSupply = Decimal(shareSupply).plus(VIRTUAL_SHARES).toString();
    const effectiveEquity = Decimal(equity).plus(VIRTUAL_ASSETS).toString();
    if (effectiveSupply === "0") return "0";
    return Decimal(userVaultShares).mul(effectiveEquity).div(effectiveSupply).toString();
  }, [userVaultShares, vaultState]);

  return {
    vaultState,
    userVaultShares,
    userSharesValue,
  };
}
