import type { AnyCoin, CoinGeckoId, Denom, Funds, Prettify } from "@left-curve/dango/types";
import { type FormatNumberOptions, formatNumber, formatUnits } from "@left-curve/dango/utils";
import { useQuery } from "@tanstack/react-query";
import { useConfig } from "./useConfig.js";

import { createStorage } from "@left-curve/store";
import type { Storage } from "@left-curve/store/types";

export type UsePricesParameters = {
  refetchInterval?: number;
  formatter?: (amount: number, options: FormatNumberOptions) => string;
  currencies?: string[];
  defaultCurrency?: string;
  defaultFormatOptions?: FormatNumberOptions;
  coins?: Record<Denom, AnyCoin>;
  storage?: Storage<{ prices: Prices }>;
};

type Prices = Record<Denom, Prettify<AnyCoin & { prices: Record<string, number> }>>;

type FormatOptions<T> = {
  formatOptions?: FormatNumberOptions;
  currency?: string;
  format?: T;
};

export function usePrices(parameters: UsePricesParameters = {}) {
  const {
    defaultCurrency = "USD",
    currencies = ["USD", "EUR"],
    refetchInterval = 60 * 1000 * 5,
    formatter = formatNumber,
    defaultFormatOptions = {
      maximumFractionDigits: 2,
      minFractionDigits: 2,
      language: navigator.language,
    },
    storage = createStorage<{ prices: Prices }>({
      key: "cache_query",
      storage:
        typeof window !== "undefined" && window.localStorage ? window.localStorage : undefined,
    }),
  } = parameters;
  const config = useConfig();

  const coins = parameters.coins || config.coins[config.state.chainId];

  function getPrice<T extends boolean = false>(
    amount: number | string,
    denom: string,
    options?: FormatOptions<T>,
  ): T extends true ? string : number {
    const {
      currency = defaultCurrency,
      formatOptions = defaultFormatOptions,
      format = false,
    } = options || {};
    const price = (() => {
      const indexCurrency = currency.toLowerCase();
      if (!data || !data?.[denom]?.prices?.[indexCurrency]) return 0;
      return Number(amount) * data[denom].prices[indexCurrency];
    })();

    return (format ? formatter(price, { ...formatOptions, currency }) : price) as T extends true
      ? string
      : number;
  }

  function calculateBalance<T extends boolean = false>(
    balances: Funds,
    options?: FormatOptions<T>,
  ): T extends true ? string : number {
    const {
      currency = defaultCurrency,
      formatOptions = defaultFormatOptions,
      format = false,
    } = options || {};
    const totalValue = Object.entries(balances).reduce((total, [denom, amount]) => {
      const price = getPrice(formatUnits(amount, coins[denom].decimals), denom, {
        currency,
        formatOptions,
        format: false,
      });
      total += price;
      return total;
    }, 0);
    return (
      format ? formatter(totalValue, { ...formatOptions, currency }) : totalValue
    ) as T extends true ? string : number;
  }

  const { data, ...rest } = useQuery<Prices>({
    queryKey: ["prices", coins, currencies],
    enabled: typeof window !== "undefined" && window.location.protocol === "https:",
    queryFn: async () => {
      const coinsByCoingeckoId = Object.fromEntries(
        Object.values(coins).map((c) => [c.coingeckoId, c]),
      );

      const response = await fetch(
        `https://api.coingecko.com/api/v3/simple/price?ids=${Object.keys(coinsByCoingeckoId).join(",")}&vs_currencies=${currencies.join(",")}`,
      );
      const parsedResponse: Record<CoinGeckoId, Record<string, number>> = await response.json();

      const prices: Prices = Object.entries(parsedResponse).reduce((acc, [coingeckoId, prices]) => {
        const coin = coinsByCoingeckoId[coingeckoId];
        if (coin) acc[coin.denom] = { ...coin, prices: prices };
        return acc;
      }, Object.create({}));

      storage.setItem("prices", prices);
      return prices;
    },
    initialData: storage.getItem("prices", {}) as Prices,
    refetchInterval,
  });

  return { data, ...rest, calculateBalance, getPrice };
}
