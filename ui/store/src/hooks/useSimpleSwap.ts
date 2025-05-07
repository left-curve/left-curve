import { useQuery } from "@tanstack/react-query";
import type { AnyCoin } from "../types/coin.js";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";

export type UseSimpleSwapParameters = {
  baseSymbol: string;
  quoteSymbol: string;
};

export function useSimpleSwap(parameters: UseSimpleSwapParameters) {
  const { baseSymbol, quoteSymbol } = parameters;
  const { coins } = useConfig();
  const { data: config, ...pairs } = useAppConfig();

  const statistics = useQuery({
    queryKey: ["pair_statistics"],
    initialData: { tvl: "-", apy: "-", volume: "-" },
    queryFn: () => {
      return { tvl: "-", apy: "-", volume: "-" };
    },
  });

  const quote = useQuery({
    queryKey: ["simpleswap_quote", quoteSymbol] as const,
    queryFn: () => {
      const coin = coins[quoteSymbol];
      if (!coin) Object;
      return coin;
    },
  });

  return {
    pairs: { ...pairs, data: config?.pairs || {} },
    statistics,
    quote: {
      coin: {} as AnyCoin,
    },
  };
}
