import { Direction } from "../types/dex.js";
import { Decimal } from "@left-curve/sdk/utils";

import type { Trade } from "../types/dex.js";
import type { WithAmount, WithDecimals, WithPrice } from "../types/utils.js";
import { formatNumber, type FormatNumberOptions, formatUnits, parseUnits } from "./formatters.js";

export function formatOrderId(id: string) {
  return Decimal(id).gte("9223372036854775807")
    ? Decimal("18446744073709551615").minus(id).toString()
    : id;
}

export function calculateTradeSize(trade: Trade, decimals: number) {
  if (trade.direction === Direction.Buy) {
    return Decimal(trade.filledBase).div(Decimal(10).pow(decimals));
  }

  return Decimal(trade.filledQuote).div(trade.clearingPrice).div(Decimal(10).pow(decimals));
}

export function calculateFees(
  base: WithAmount<WithDecimals<WithPrice>>,
  quote: WithAmount<WithDecimals<WithPrice>>,
  options: FormatNumberOptions,
) {
  const baseFee = Decimal(formatUnits(base.amount, base.decimals)).mul(base.price);
  const quoteFee = Decimal(formatUnits(quote.amount, quote.decimals)).mul(quote.price);

  return formatNumber(baseFee.plus(quoteFee).toFixed(), { ...options, currency: "USD" });
}

export function calculatePrice(
  price: string,
  decimals: { base: number; quote: number },
  options: FormatNumberOptions,
) {
  return formatNumber(parseUnits(price, decimals.base - decimals.quote), {
    ...options,
    minSignificantDigits: 8,
    maxSignificantDigits: 8,
  }).slice(0, 7);
}
