import { useMemo } from "react";

type UsePerpsMaxSizeParameters = {
  availableMargin: number;
  leverage: number;
  currentPrice: number;
  /** Taker fee rate as a decimal (e.g. 0.00045 for 0.045%). */
  takerFeeRate: number;
  isBaseSize: boolean;
};

/**
 * Compute the maximum order size for a perps trade.
 *
 * The chain reserves both the initial margin AND the taker fee up-front, so the
 * usable notional is:
 *
 *     max_size_in_usd  = available_margin / ((1 / leverage) + taker_fee_rate)
 *     max_size_in_asset = max_size_in_usd / oracle_price
 */
export function usePerpsMaxSize(parameters: UsePerpsMaxSizeParameters) {
  const { availableMargin, leverage, currentPrice, takerFeeRate, isBaseSize } = parameters;

  return useMemo(() => {
    if (availableMargin <= 0 || currentPrice <= 0 || leverage <= 0) return 0;
    const denominator = 1 / leverage + Math.max(takerFeeRate, 0);
    if (denominator <= 0) return 0;
    const maxNotional = availableMargin / denominator;
    return isBaseSize ? maxNotional / currentPrice : maxNotional;
  }, [availableMargin, leverage, currentPrice, takerFeeRate, isBaseSize]);
}
