/**
 * Pure math helpers for converting between an absolute TP/SL trigger price
 * and the ROI% it represents on the trader's capital.
 *
 * The key insight is that "ROI% on capital" is not the same as "% change in
 * price" — at L× leverage, a P% price move corresponds to a (P × L)% change
 * in the margin the trader has locked in the position:
 *
 *     ROI% ≈ price_change% × leverage
 *
 * This matches how Binance / Bybit / Hyperliquid / dYdX display TP/SL ROI.
 * See the plan at `.claude/plans/foamy-kindling-dongarra.md` and the
 * industry references it cites.
 *
 * These helpers are pure — no React, no side effects — so they can be
 * exhaustively unit-tested without a renderer.
 */

export type TpslKind = "tp" | "sl";

export type TpslMathInput = {
  /** Reference price to compute the ROI against (entry price for an open
   *  position, limit price for a resting limit order, or current mark for
   *  a market order). */
  referencePrice: number;
  /** Effective leverage on the position. Clamped to ≥1 internally so a
   *  missing or zero value degrades gracefully to the "raw price delta"
   *  display. */
  leverage: number;
  /** True for a long position, false for a short. */
  isLong: boolean;
  /** Whether the trigger is a take-profit or stop-loss. */
  kind: TpslKind;
};

/**
 * True if the trigger price is expected to sit *above* the reference price
 * for this (side, kind) combination. Long TPs and short SLs are "upside"
 * triggers; long SLs and short TPs are "downside" triggers.
 */
function isUpsideTrigger({ isLong, kind }: Pick<TpslMathInput, "isLong" | "kind">): boolean {
  return (isLong && kind === "tp") || (!isLong && kind === "sl");
}

/**
 * Clamp leverage to ≥1. A zero, negative, or non-finite leverage would
 * otherwise produce nonsensical ROI values (division/multiplication by
 * zero, sign flips). Clamping to 1 means the ROI display degrades to
 * "raw price delta %" — the same behavior as before leverage was wired in.
 */
function clampLeverage(leverage: number): number {
  return Number.isFinite(leverage) && leverage >= 1 ? leverage : 1;
}

/**
 * Compute the signed ROI% on capital implied by a trigger price.
 *
 * Returns a *signed* number: positive means the trigger is on the "correct"
 * side of the reference (i.e. would pay out on a TP or take the loss on an
 * SL as intended), negative means the trigger is on the wrong side (e.g. a
 * long TP below the reference) and would trigger immediately. Callers
 * typically clamp this to ≥0 for display; we leave the sign in for testing
 * and so invalid inputs are distinguishable.
 *
 * Returns 0 for non-finite inputs or a non-positive reference price.
 */
export function roiFromPrice(
  triggerPrice: number,
  { referencePrice, leverage, isLong, kind }: TpslMathInput,
): number {
  if (!Number.isFinite(triggerPrice) || !Number.isFinite(referencePrice) || referencePrice <= 0) {
    return 0;
  }
  const L = clampLeverage(leverage);
  const priceDeltaPct = isUpsideTrigger({ isLong, kind })
    ? (triggerPrice - referencePrice) / referencePrice
    : (referencePrice - triggerPrice) / referencePrice;
  return priceDeltaPct * 100 * L;
}

/**
 * Compute the absolute trigger price implied by an ROI% on capital.
 *
 * Inverse of {@link roiFromPrice} for any finite `roiPct` when the reference
 * price is strictly positive and leverage is finite and ≥1:
 *
 *     roiFromPrice(priceFromRoi(r, x), x) === r
 *
 * (Subject to floating-point rounding; tests allow a tiny epsilon.)
 *
 * Returns the reference price unchanged for non-finite `roiPct` or a
 * non-positive reference price.
 */
export function priceFromRoi(
  roiPct: number,
  { referencePrice, leverage, isLong, kind }: TpslMathInput,
): number {
  if (!Number.isFinite(roiPct) || !Number.isFinite(referencePrice) || referencePrice <= 0) {
    return referencePrice;
  }
  const L = clampLeverage(leverage);
  const priceDeltaPct = roiPct / 100 / L;
  return isUpsideTrigger({ isLong, kind })
    ? referencePrice * (1 + priceDeltaPct)
    : referencePrice * (1 - priceDeltaPct);
}
