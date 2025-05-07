import type { Denom, Funds, Price } from "@left-curve/dango/types";
import { type FormatNumberOptions, formatNumber, formatUnits } from "@left-curve/dango/utils";
import { useQuery } from "@tanstack/react-query";
import { useConfig } from "./useConfig.js";

import type { AnyCoin } from "../types/coin.js";
import { usePublicClient } from "./usePublicClient.js";

export type UsePricesParameters = {
  refetchInterval?: number;
  formatter?: (amount: number, options: FormatNumberOptions) => string;
  defaultFormatOptions?: FormatNumberOptions;
  coins?: Record<Denom, AnyCoin>;
};

type FormatOptions<T> = {
  formatOptions?: FormatNumberOptions;
  format?: T;
};

export function usePrices(parameters: UsePricesParameters = {}) {
  const client = usePublicClient();

  const {
    refetchInterval = 60 * 1000 * 5,
    formatter = formatNumber,
    defaultFormatOptions = {
      maximumFractionDigits: 2,
      minFractionDigits: 2,
      language: navigator.language,
      mask: 1,
    },
  } = parameters;
  const config = useConfig();

  const coins = parameters.coins || config.coins;

  function getPrice<T extends boolean = false>(
    amount: number | string,
    denom: string,
    options?: FormatOptions<T>,
  ): T extends true ? string : number {
    const { formatOptions = defaultFormatOptions, format = false } = options || {};

    const price = (() => {
      if (!prices || !prices?.[denom]?.humanizedPrice) return 0;
      return Number(amount) * Number(prices[denom].humanizedPrice);
    })();

    return (
      format ? formatter(price, { ...formatOptions, currency: "usd" }) : price
    ) as T extends true ? string : number;
  }

  function calculateBalance<T extends boolean = false>(
    balances: Funds,
    options?: FormatOptions<T>,
  ): T extends true ? string : number {
    const { formatOptions = defaultFormatOptions, format = false } = options || {};
    const totalValue = Object.entries(balances).reduce((total, [denom, amount]) => {
      const price = getPrice(formatUnits(amount, coins[denom].decimals), denom, {
        formatOptions,
        format: false,
      });
      total += price;
      return total;
    }, 0);
    return (format ? formatter(totalValue, { ...formatOptions }) : totalValue) as T extends true
      ? string
      : number;
  }

  const { data: prices, ...rest } = useQuery({
    queryKey: ["prices", coins],
    queryFn: async () => {
      const prices = await client.getPrices();

      return Object.entries(prices).reduce((acc, [denom, coin]) => {
        if (denom.includes("usdc")) acc["hyp/eth/usdc"] = coin;
        else acc[`hyp/all/${denom.split("/")[2]}`] = coin;
        return acc;
      }, Object.create({})) as Record<Denom, Price>;
    },
    staleTime: 1000 * 60 * 5,
    refetchInterval,
  });

  return { prices, ...rest, calculateBalance, getPrice };
}
