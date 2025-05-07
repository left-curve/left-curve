import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";

import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";

import type { AnyCoin } from "../types/coin.js";

const BASE_DENOM = "USDC";

export type UseSimpleSwapParameters = {
  pair: { from: string; to: string };
  onChangePair: (pair: { from: string; to: string }) => void;
};

export function useSimpleSwap(parameters: UseSimpleSwapParameters) {
  const { onChangePair } = parameters;
  const { from, to } = parameters.pair;
  const { coins } = useConfig();
  const { data: config, ...pairs } = useAppConfig();

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

  const statistics = useQuery({
    queryKey: ["pair_statistics"],
    initialData: { tvl: "-", apy: "-", volume: "-" },
    queryFn: () => {
      return { tvl: "-", apy: "-", volume: "-" };
    },
  });

  return {
    coins,
    pairs: { ...pairs, data: config?.pairs || {} },
    statistics,
    quote: quoteCoin,
    base: baseCoin,
    isReverse,
    direction,
    toggleDirection,
    changeQuote,
  };
}
