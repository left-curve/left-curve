import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import { useAccount } from "./useAccount.js";
import { useBalances } from "./useBalances.js";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";
import { useSigningClient } from "./useSigningClient.js";
import { useSubmitTx } from "./useSubmitTx.js";

import { formatUnits, parseUnits } from "@left-curve/dango/utils";
import Big from "big.js";

import type { PairUpdate } from "@left-curve/dango/types";

export type UsePoolLiquidityStateParameters = {
  pair: PairUpdate;
  action: "deposit" | "withdraw";
  onChangeAction: (action: "deposit" | "withdraw") => void;
  controllers: {
    inputs: Record<string, { value: string }>;
    reset: () => void;
    setValue: (name: string, value: string) => void;
  };
};

export function usePoolLiquidityState(parameters: UsePoolLiquidityStateParameters) {
  const { pair, action, controllers, onChangeAction } = parameters;
  const { inputs } = controllers;
  const publicClient = usePublicClient();
  const { account } = useAccount();
  const { coins } = useConfig();

  const poolRate = 1;

  const { data: signingClient } = useSigningClient();

  const { data: balances = {}, refetch: updateBalance } = useBalances({
    address: account?.address,
  });

  const baseCoin = coins[pair.baseDenom];
  const quoteCoin = coins[pair.quoteDenom];

  const lpDenom = `dex/pool${baseCoin.denom.replace("bridge", "")}${quoteCoin.denom.replace("bridge", "")}`;

  const baseAmount = inputs.baseAmount?.value || "0";
  const quoteAmount = inputs.quoteAmount?.value || "0";

  const withdrawPercent = inputs.withdrawPercent?.value || "0";

  const lpBalance = useMemo(() => {
    return balances[lpDenom] || "0";
  }, [balances, baseCoin, quoteCoin]);

  const userHasLiquidity = lpBalance !== "0";

  const baseBalance = useMemo(
    () => formatUnits(balances[baseCoin.denom] || "0", baseCoin.decimals),
    [balances, baseCoin],
  );
  const quoteBalance = useMemo(
    () => formatUnits(balances[quoteCoin.denom] || "0", quoteCoin.decimals),
    [balances, quoteCoin],
  );

  useEffect(() => {
    const amount = String(+baseAmount * poolRate);
    if (amount === quoteAmount) return;
    controllers.setValue("quoteAmount", amount);
  }, [baseAmount]);

  useEffect(() => {
    const amount = String(+quoteAmount * poolRate);
    if (amount === baseAmount) return;
    controllers.setValue("baseAmount", amount);
  }, [quoteAmount]);

  const userLiquidity = useQuery({
    enabled: userHasLiquidity,
    queryKey: ["userLiquidity", account?.address, pair.baseDenom, pair.quoteDenom],
    queryFn: async () => {
      if (!account) throw new Error("not account found");
      const [{ amount: baseAmount }, { amount: quoteAmount }] =
        await publicClient.simulateWithdrawLiquidity({
          baseDenom: pair.baseDenom,
          quoteDenom: pair.quoteDenom,
          lpBurnAmount: lpBalance,
        });
      const baseParseAmount = formatUnits(baseAmount, baseCoin.decimals);
      const quoteParseAmount = formatUnits(quoteAmount, quoteCoin.decimals);

      return {
        innerBase: baseParseAmount,
        innerQuote: quoteParseAmount,
      };
    },
  });

  const deposit = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("signingClient not available");
        if (!account) throw new Error("not account found");

        await signingClient.provideLiquidity({
          sender: account.address,
          baseDenom: pair.baseDenom,
          quoteDenom: pair.quoteDenom,
          funds: {
            [baseCoin.denom]: parseUnits(baseAmount, baseCoin.decimals).toString(),
            [quoteCoin.denom]: parseUnits(quoteAmount, quoteCoin.decimals).toString(),
          },
        });
      },
      onSuccess: () => {
        updateBalance();
        controllers.reset();
      },
    },
  });

  const withdraw = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("signingClient not available");
        if (!account) throw new Error("not account found");

        await signingClient.withdrawLiquidity({
          sender: account.address,
          baseDenom: pair.baseDenom,
          quoteDenom: pair.quoteDenom,
          funds: {
            [lpDenom]: Big(lpBalance).mul(withdrawPercent).div(100).toFixed(0),
          },
        });
      },
      onSuccess: () => {
        updateBalance();
        controllers.reset();
      },
    },
  });

  const withdrawAmount = useMemo(() => {
    if (!userLiquidity.data) return { base: "0", quote: "0" };
    const baseAmount = Big(userLiquidity.data.innerBase).mul(withdrawPercent).div(100).toNumber();
    const quoteAmount = Big(userLiquidity.data.innerQuote).mul(withdrawPercent).div(100).toNumber();
    return {
      base: baseAmount,
      quote: quoteAmount,
    };
  }, [withdrawPercent, userLiquidity.data]);

  return {
    pair,
    action,
    onChangeAction,
    userLiquidity,
    userHasLiquidity,
    withdrawPercent,
    withdrawAmount,
    deposit,
    withdraw,
    coins: {
      base: {
        ...baseCoin,
        balance: baseBalance,
        amount: baseAmount,
      },
      quote: {
        ...quoteCoin,
        balance: quoteBalance,
        amount: quoteAmount,
      },
    },
  };
}
