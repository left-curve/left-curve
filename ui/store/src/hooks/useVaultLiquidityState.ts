import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { useAccount } from "./useAccount.js";
import { useAppConfig } from "./useAppConfig.js";
import { usePublicClient } from "./usePublicClient.js";
import { useSigningClient } from "./useSigningClient.js";
import { useSubmitTx } from "./useSubmitTx.js";
import { perpsUserStateStore } from "./usePerpsUserState.js";
import { perpsUserStateExtendedStore } from "./usePerpsUserStateExtended.js";

import { Decimal } from "@left-curve/dango/utils";

const VIRTUAL_SHARES = "1000000";
const VIRTUAL_ASSETS = "1";

export type UseVaultLiquidityStateParameters = {
  action: "deposit" | "withdraw";
  onChangeAction: (action: "deposit" | "withdraw") => void;
  controllers: {
    inputs: Record<string, { value: string }>;
    reset: () => void;
    setValue: (name: string, value: string) => void;
  };
};

export function useVaultLiquidityState(parameters: UseVaultLiquidityStateParameters) {
  const { action, controllers, onChangeAction } = parameters;
  const { inputs } = controllers;
  const publicClient = usePublicClient();
  const { account } = useAccount();
  const { data: appConfig } = useAppConfig();

  const { data: signingClient } = useSigningClient();

  const perpsUserState = perpsUserStateStore((s) => s.userState);
  const availableMargin = perpsUserStateExtendedStore((s) => s.availableMargin);

  const depositAmount = Decimal(inputs.depositAmount?.value || "0").toFixed(6);
  const withdrawShares = Decimal(inputs.withdrawShares?.value || "0").toFixed(0);

  const vaultState = useQuery({
    queryKey: ["vaultState"],
    queryFn: async () => {
      return await publicClient.getPerpsVaultState();
    },
    refetchInterval: 10_000,
  });

  const userVaultShares = account ? (perpsUserState?.vaultShares ?? "0") : "0";
  const userMargin = account ? (availableMargin ?? "0") : "0";
  const userUnlocks = account ? (perpsUserState?.unlocks ?? []) : [];
  const userHasShares = account ? userVaultShares !== "0" : false;

  const shareSupply = vaultState.data?.shareSupply ?? "0";
  const equity = vaultState.data?.equity ?? "0";
  const isPaused = !(vaultState.data?.depositWithdrawalActive ?? true);

  const vaultMargin = vaultState.data?.margin ?? "0";
  const vaultDepositCap = appConfig.perpsParam.vaultDepositCap;
  const isTvlCapReached = vaultDepositCap != null && Decimal(vaultMargin).gte(vaultDepositCap);

  const effectiveSupply = useMemo(
    () => Decimal(shareSupply).plus(VIRTUAL_SHARES).toString(),
    [shareSupply],
  );

  const effectiveEquity = useMemo(() => Decimal(equity).plus(VIRTUAL_ASSETS).toString(), [equity]);

  const sharePrice = useMemo(() => {
    if (shareSupply === "0") return "0";
    return Decimal(equity).div(shareSupply).toString();
  }, [equity, shareSupply]);

  const sharesToReceive = useMemo(() => {
    if (depositAmount === "0" || effectiveEquity === "0") return "0";
    return Decimal(depositAmount).mul(effectiveSupply).div(effectiveEquity).toFixed(0);
  }, [depositAmount, effectiveSupply, effectiveEquity]);

  const usdToReceive = useMemo(() => {
    if (withdrawShares === "0" || effectiveSupply === "0") return "0";
    return Decimal(withdrawShares).mul(effectiveEquity).div(effectiveSupply).toString();
  }, [withdrawShares, effectiveEquity, effectiveSupply]);

  const userSharesValue = useMemo(() => {
    if (userVaultShares === "0" || effectiveSupply === "0") return "0";
    return Decimal(userVaultShares).mul(effectiveEquity).div(effectiveSupply).toString();
  }, [userVaultShares, effectiveEquity, effectiveSupply]);

  const deposit = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("signingClient not available");
        if (!account) throw new Error("no account found");

        await signingClient.vaultAddLiquidity({
          sender: account.address,
          amount: depositAmount,
        });
      },
      invalidateKeys: [["vaultState"]],
      onSuccess: () => {
        controllers.reset();
        controllers.setValue("depositAmount", "");
      },
    },
  });

  const withdraw = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("signingClient not available");
        if (!account) throw new Error("no account found");

        await signingClient.vaultRemoveLiquidity({
          sender: account.address,
          sharesToBurn: withdrawShares,
        });
      },
      invalidateKeys: [["vaultState"]],
      onSuccess: () => {
        controllers.reset();
        controllers.setValue("withdrawShares", "");
      },
    },
  });

  return {
    action,
    onChangeAction,
    isPaused,
    isTvlCapReached,
    vaultState: vaultState.data,
    isLoading: vaultState.isLoading,
    userVaultShares,
    userSharesValue,
    userMargin,
    userUnlocks,
    userHasShares,
    sharePrice,
    sharesToReceive,
    usdToReceive,
    deposit,
    withdraw,
  };
}
