import { useQuery } from "@tanstack/react-query";
import { useMemo, useRef, useState } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";

import type { Coin, PairUpdate } from "@left-curve/dango/types";
import { formatUnits } from "@left-curve/dango/utils";
import type { AnyCoin } from "../types/coin.js";
import { usePrices } from "./usePrices.js";

const BASE_DENOM = "USDC";

export type UseSimpleSwapParameters = {
  pair: { from: string; to: string };
  onChangePair: (pair: { from: string; to: string }) => void;
};

export type SimpleSwapInfo = {
  input: Coin;
  pair: PairUpdate;
  priceImpact: number;
  fee: number;
};

export function useSimpleSwap(parameters: UseSimpleSwapParameters) {
  const { onChangePair } = parameters;
  const { from, to } = parameters.pair;
  const { coins } = useConfig();
  const { data: config, ...pairs } = useAppConfig();
  const { getPrice } = usePrices();

  const client = usePublicClient();

  const simulationInput = useRef<Coin | null>(null);

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

  const simulation = useQuery({
    enabled: false,
    queryKey: ["pair_simulation"],
    queryFn: async () => {
      if (!simulationInput.current || !pair) return null;
      return await client.simulateSwapExactAmountIn({
        input: simulationInput.current,
        route: [{ baseDenom: pair.baseDenom, quoteDenom: pair.quoteDenom }],
      });
    },
  });

  const simulate = async (input: Coin) => {
    simulationInput.current = input;
    const { data } = await simulation.refetch();
    return data;
  };

  const fee = useMemo(() => {
    if (!simulationInput.current || !simulation.data) return 0;
    return (
      Math.round(
        getPrice(
          formatUnits(
            simulationInput.current.amount,
            coins[simulationInput.current.denom].decimals,
          ),
          simulationInput.current.denom,
        ),
      ) * Number(pair?.params.swapFeeRate)
    );
  }, [simulation.data]);

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
      input: simulationInput.current,
      ...simulation,
    },
  };
}
