import { useMutation, useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { usePrices } from "./usePrices.js";
import { usePublicClient } from "./usePublicClient.js";

import { formatUnits } from "@left-curve/dango/utils";

import type { Coin, PairUpdate } from "@left-curve/dango/types";
import type { AnyCoin } from "../types/coin.js";

const BASE_DENOM = "USDC";

export type UseSimpleSwapStateParameters = {
  pair: { from: string; to: string };
  onChangePair: (pair: { from: string; to: string }) => void;
};

export type SimpleSwapInfo = {
  input: Coin;
  pair: PairUpdate;
  priceImpact: number;
  fee: number;
};

export function useSimpleSwapState(parameters: UseSimpleSwapStateParameters) {
  const { onChangePair } = parameters;
  const { from, to } = parameters.pair;
  const { coins } = useConfig();
  const { data: config, ...pairs } = useAppConfig();
  const { getPrice } = usePrices();

  const client = usePublicClient();

  const changeQuote = (quote: string) => {
    const newPair = isReverse ? { from: quote, to } : { from, to: quote };
    onChangePair(newPair);
  };

  const [direction, setDirection] = useState<"reverse" | "normal">(
    from === BASE_DENOM ? "normal" : "reverse",
  );

  const toggleDirection = () => {
    const newPair = { from: to, to: from };
    onChangePair(newPair);
    setDirection(isReverse ? "normal" : "reverse");
  };

  const isReverse = direction === "reverse";

  const coinsBySymbol: Record<string, AnyCoin> = useMemo(
    () =>
      Object.values(coins).reduce((acc, coin) => {
        acc[coin.symbol] = coin;
        return acc;
      }, Object.create({})),
    [coins],
  );

  const baseCoin = coinsBySymbol[isReverse ? to : from];
  const quoteCoin = coinsBySymbol[isReverse ? from : to];

  const pair = config?.pairs?.[quoteCoin.denom];

  const statistics = useQuery({
    queryKey: ["pair_statistics"],
    initialData: { tvl: "-", apy: "-", volume: "-" },
    queryFn: () => {
      return { tvl: "-", apy: "-", volume: "-" };
    },
  });

  const {
    mutateAsync: simulate,
    mutate: _,
    ...simulation
  } = useMutation({
    mutationFn: async (operation: { input: Coin; pair: PairUpdate }) => {
      const { input, pair } = operation;
      const output = await client.simulateSwapExactAmountIn({
        input,
        route: [{ baseDenom: pair.baseDenom, quoteDenom: pair.quoteDenom }],
      });

      return { input, output };
    },
  });

  const fee = useMemo(() => {
    if (!simulation.data || !pair) return 0;
    const { output } = simulation.data;
    return (
      Math.round(getPrice(formatUnits(output.amount, coins[output.denom].decimals), output.denom)) *
      Number(pair.params.swapFeeRate)
    );
  }, [pair, simulation.data]);

  return {
    coins,
    pair,
    pairs: { ...pairs, data: config?.pairs || {} },
    statistics,
    quote: quoteCoin,
    base: baseCoin,
    isReverse,
    direction,
    fee,
    toggleDirection,
    changeQuote,
    simulation: {
      simulate,
      ...simulation,
    },
  };
}
