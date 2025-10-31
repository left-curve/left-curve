import { useMutation, useQuery } from "@tanstack/react-query";
import { startTransition, useMemo, useState } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { usePrices } from "./usePrices.js";
import { usePublicClient } from "./usePublicClient.js";

import { formatUnits, parseUnits } from "@left-curve/dango/utils";

import type { Address, Coin, PairUpdate } from "@left-curve/dango/types";
import { useSubmitTx } from "./useSubmitTx.js";
import { useAccount } from "./useAccount.js";
import { useSigningClient } from "./useSigningClient.js";

export type UseConvertStateParameters = {
  pair: { from: string; to: string };
  controllers: {
    inputs: Record<string, { value: string }>;
    reset: () => void;
    setValue: (name: string, value: string) => void;
  };
  onChangePair: (pair: { from: string; to: string }) => void;
  submission: {
    onError: (error: unknown) => void;
    confirm: () => Promise<void>;
  };
  simulation: {
    onError: (error: unknown) => void;
  };
};

export type ConvertInfo = {
  input: Coin;
  pair: PairUpdate;
  priceImpact: number;
  fee: number;
};

export function useConvertState(parameters: UseConvertStateParameters) {
  const { onChangePair, controllers } = parameters;
  const { inputs, setValue } = controllers;
  const { from, to } = parameters.pair;
  const { coins } = useConfig();
  const { account } = useAccount();
  const client = usePublicClient();
  const { data: config } = useAppConfig();
  const { data: signingClient } = useSigningClient();

  const { getPrice } = usePrices();

  const pairId = useMemo(
    () => ({
      base: from === "USDC" ? coins.bySymbol[to] : coins.bySymbol[from],
      quote: from === "USDC" ? coins.bySymbol[from] : coins.bySymbol[to],
    }),
    [from, to],
  );

  const pair = config?.pairs[pairId.base.denom];

  const changePair = (symbol: string) => {
    const newPair = isReverse ? { from: symbol, to } : { from, to: symbol };
    onChangePair(newPair);
  };

  const [direction, setDirection] = useState<"reverse" | "normal">(
    from === "USDC" ? "normal" : "reverse",
  );

  const toggleDirection = () => {
    startTransition(() => {
      const newPair = { from: to, to: from };
      onChangePair(newPair);
      setDirection(isReverse ? "normal" : "reverse");
    });
  };

  const isReverse = direction === "reverse";

  const fromCoin = coins.bySymbol[from];
  const toCoin = coins.bySymbol[to];

  const statistics = useQuery({
    queryKey: ["pair_statistics"],
    initialData: { tvl: "-", apy: "-", volume: "-" },
    queryFn: () => {
      return { tvl: "-", apy: "-", volume: "-" };
    },
  });

  const simulation = useMutation({
    onError: (e, direction) => {
      setValue(direction === "from" ? "to" : "from", "0");
      parameters.simulation.onError(e);
    },
    mutationFn: async (direction: "from" | "to") => {
      const fromAmount = inputs.from?.value || "0";
      const toAmount = inputs.to?.value || "0";

      if (direction === "from") {
        const input = {
          denom: fromCoin.denom,
          amount: parseUnits(fromAmount, fromCoin.decimals),
        };

        if (fromAmount === "0") {
          setValue("to", "0");
          return { input, output: { denom: toCoin.denom, amount: "0" } };
        }

        const output = await client.simulateSwapExactAmountIn({
          input,
          route: [{ baseDenom: pairId.base.denom, quoteDenom: pairId.quote.denom }],
        });

        setValue("to", formatUnits(output.amount, toCoin.decimals));
        return { input, output };
      }

      const output = {
        denom: toCoin.denom,
        amount: parseUnits(toAmount, toCoin.decimals),
      };

      if (toAmount === "0") {
        setValue("from", "0");
        return { input: { denom: fromCoin.denom, amount: "0" }, output };
      }

      const input = await client.simulateSwapExactAmountOut({
        output,
        route: [{ baseDenom: pairId.base.denom, quoteDenom: pairId.quote.denom }],
      });

      setValue("from", formatUnits(input.amount, fromCoin.decimals));

      return { input, output };
    },
  });

  const fee = useMemo(() => {
    if (!simulation.data || !pair) return 0;
    const { output } = simulation.data;
    return (
      Math.round(
        getPrice(formatUnits(output.amount, coins.byDenom[output.denom].decimals), output.denom),
      ) * Number(pair.params.swapFeeRate)
    );
  }, [pair, simulation.data]);

  const submission = useSubmitTx({
    mutation: {
      invalidateKeys: [["quests", account?.username]],
      mutationFn: async (_, { abort }) => {
        if (!signingClient) throw new Error("error: no signing client");
        if (!pair) throw new Error("error: no pair");
        if (!simulation.data) throw new Error("error: no simulation data");

        const { input } = simulation.data;

        await parameters.submission.confirm().catch(abort);

        await signingClient.swapExactAmountIn({
          sender: account!.address as Address,
          route: [{ baseDenom: pair.baseDenom, quoteDenom: pair.quoteDenom }],
          input: {
            denom: input.denom,
            amount: input.amount,
          },
        });
      },
      onSuccess: () => {
        controllers.reset();
        simulation.reset();
      },
    },
  });

  return {
    coins,
    pair,
    pairId,
    statistics,
    fromCoin,
    toCoin,
    isReverse,
    direction,
    fee,
    toggleDirection,
    changePair,
    submission,
    simulation,
  };
}
