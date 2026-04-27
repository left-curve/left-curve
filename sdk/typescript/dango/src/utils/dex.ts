import { Direction } from "../types/dex.js";
import { Decimal } from "@left-curve/sdk/utils";

import type { Trade } from "../types/dex.js";
import type { RateSchedule } from "../types/perps.js";
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
  return formatNumber(parseUnits(price, decimals.base - decimals.quote), options);
}

/**
 * Resolves the applicable rate from a RateSchedule given a user's volume.
 * Mirrors the backend RateSchedule::resolve logic: finds the highest tier
 * threshold that the volume meets or exceeds, falling back to the base rate.
 */
export function resolveRateSchedule(schedule: RateSchedule, volume: string): string {
  const vol = Decimal(volume);
  const applicableTier = Object.entries(schedule.tiers)
    .filter(([threshold]) => !Decimal.isNaN(threshold) && vol.gte(Decimal(threshold)))
    .reduce<readonly [string, string] | null>(
      (highest, [threshold, rate]) =>
        !highest || Decimal(threshold).gt(highest[0]) ? [threshold, rate] : highest,
      null,
    );

  return applicableTier ? applicableTier[1] : schedule.base;
}

export function adjustPrice(price: number, shifting = 6, min = 2) {
  let decimalDigits: number;

  if (price > 1.0) {
    const integerDigits = Math.floor(Math.log10(price)) + 1;
    decimalDigits = Math.max(shifting - integerDigits, min);
  } else {
    const leadingZeros = Math.floor(Math.log10(1 / price));
    decimalDigits = Math.max(shifting + leadingZeros, min);
  }

  const multiplier = 10 ** decimalDigits;
  return Math.floor(price * multiplier) / multiplier;
}
