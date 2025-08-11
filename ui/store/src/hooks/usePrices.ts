import { useQuery } from "@tanstack/react-query";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";

import { Decimal, formatNumber, formatUnits, parseUnits } from "@left-curve/dango/utils";

import type { Denom, Funds, Price } from "@left-curve/dango/types";
import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { AnyCoin } from "../types/coin.js";

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

  const coins = parameters.coins || config.coins.byDenom;

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

  function convertAmount<T extends boolean = false>(
    fromAmount: string | number,
    fromDenom: string,
    targetDenom: string,
    parse?: T,
  ): T extends false ? number : string {
    const fromPrice = getPrice(fromAmount, fromDenom);
    const targetPrice = getPrice(1, targetDenom);

    const targetAmount = Decimal(fromPrice).div(targetPrice).toFixed();

    return (
      parse ? parseUnits(targetAmount, coins[targetDenom].decimals).toString() : targetAmount
    ) as T extends false ? number : string;
  }

  function calculateBalance<T extends boolean = false>(
    balances: Funds,
    options?: FormatOptions<T>,
  ): T extends true ? string : number {
    const { formatOptions = defaultFormatOptions, format = false } = options || {};
    const totalValue = Object.entries(balances).reduce((total, [denom, amount]) => {
      const coin = coins[denom];
      const price = coin
        ? getPrice(formatUnits(amount, coin.decimals), denom, {
            formatOptions,
            format: false,
          })
        : 0;
      total += price;
      return total;
    }, 0);
    return (format ? formatter(totalValue, { ...formatOptions }) : totalValue) as T extends true
      ? string
      : number;
  }

  const { data: prices, ...rest } = useQuery<Record<Denom, Price>>({
    queryKey: ["prices", coins],
    queryFn: () => client.getPrices(),
    staleTime: 1000 * 60 * 5,
    refetchInterval,
  });

  return { prices, ...rest, calculateBalance, getPrice, convertAmount };
}
