import { useMemo } from "react";

type UsePerpsMaxSizeParameters = {
  /** User's total equity across all positions. */
  equity: number;
  /** USD margin locked up as collateral for the user's open GTC limit orders (`userState.reservedMargin`). */
  reservedMargin: number;
  /** Signed base-unit size of the current position in this pair. Positive = long, negative = short, 0 = no position. */
  currentPositionSize: number;
  /** Order direction. */
  action: "buy" | "sell";
  /** User-selected leverage (integer ≥ 1). */
  leverage: number;
  /** Mark price (from pair stats, falling back to oracle). */
  currentPrice: number;
  /** Taker fee rate as a decimal (e.g. 0.00045 for 0.045%). */
  takerFeeRate: number;
  /** Whether the reduce-only checkbox is checked. */
  reduceOnly: boolean;
  /** Whether the user is entering size in base units or USD notional. */
  isBaseSize: boolean;
};

type UsePerpsMaxSizeResult = {
  /** Side-dependent available-to-trade amount in USD. */
  availToTrade: number;
  /** Maximum order size in either base or notional units depending on isBaseSize. */
  maxSize: number;
};

/**
 * Compute the maximum order size for a perps trade, accounting for the user's
 * existing position, direction, and margin locked by open limit orders.
 *
 * Three regimes:
 *
 * 1. **Same-side (adding to position):** available margin shrinks because the
 *    existing position already locks up IM at the selected leverage.
 *    `availToTrade = equity − |pos|·mark/L − reserved`
 *
 * 2. **Opposing (closing / flipping):** available margin grows because closing
 *    releases the IM locked by the existing position.
 *    `availToTrade = equity + |pos|·mark/L − reserved`
 *
 * 3. **Reduce-only:** max is capped at the closable portion of the position
 *    (`|pos|` in base, `|pos|·mark` in notional). Leverage, fees, and reserved
 *    margin are irrelevant — the chain skips the margin check for reduce-only
 *    orders.
 */
export function usePerpsMaxSize(parameters: UsePerpsMaxSizeParameters): UsePerpsMaxSizeResult {
  const {
    equity,
    reservedMargin,
    currentPositionSize,
    action,
    leverage,
    currentPrice,
    takerFeeRate,
    reduceOnly,
    isBaseSize,
  } = parameters;

  return useMemo(() => {
    const zero: UsePerpsMaxSizeResult = { availToTrade: 0, maxSize: 0 };

    if (currentPrice <= 0 || leverage <= 0) return zero;

    const positionBase = Math.abs(currentPositionSize);
    const orderSign = action === "buy" ? 1 : -1;
    const isOpposing =
      currentPositionSize !== 0 && Math.sign(currentPositionSize) !== orderSign;

    const imPosAtL = (positionBase * currentPrice) / leverage;
    const reserved = Math.max(reservedMargin, 0);

    const availToTrade =
      positionBase === 0
        ? equity - reserved
        : isOpposing
          ? equity + imPosAtL - reserved
          : equity - imPosAtL - reserved;

    if (reduceOnly) {
      const maxBase = isOpposing ? positionBase : 0;
      const maxNotional = maxBase * currentPrice;
      return {
        availToTrade,
        maxSize: isBaseSize ? maxBase : maxNotional,
      };
    }

    const denom = 1 / leverage + Math.max(takerFeeRate, 0);
    const availPositive = Math.max(availToTrade, 0);
    const maxNotional = denom > 0 ? availPositive / denom : 0;

    return {
      availToTrade,
      maxSize: isBaseSize ? maxNotional / currentPrice : maxNotional,
    };
  }, [
    equity,
    reservedMargin,
    currentPositionSize,
    action,
    leverage,
    currentPrice,
    takerFeeRate,
    reduceOnly,
    isBaseSize,
  ]);
}
