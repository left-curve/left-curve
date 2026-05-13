import type { PerpsPositionExtended, PerpsPairParam } from "@left-curve/dango/types";

type PairPrice = { currentPrice?: string | null };

/**
 * Compute the estimated liquidation price for a position using the same
 * cross-margin formula as the on-chain `compute_liquidation_price`:
 *
 *   C = margin + otherPnl - size*entryPrice - totalFunding - otherMm
 *   liqPrice = C / (|size|*mmr - size)
 *
 * Returns `null` when the inputs are insufficient or the price is non-positive.
 */
export function computeLiquidationPrice(params: {
  margin: number;
  size: number;
  entryPrice: number;
  mmr: number;
  targetPairId: string;
  extendedPositions: Record<string, PerpsPositionExtended>;
  pairPrices: Record<string, PairPrice>;
  pairParams: Record<string, PerpsPairParam>;
}): number | null {
  const { margin, size, entryPrice, mmr, targetPairId, extendedPositions, pairPrices, pairParams } =
    params;

  if (margin <= 0 || entryPrice <= 0 || size === 0) return null;

  let otherPnl = 0;
  let totalFunding = 0;
  let otherMm = 0;

  for (const [pid, pos] of Object.entries(extendedPositions)) {
    if (pid === targetPairId) {
      // Only count funding from the target pair's existing position
      totalFunding += Number(pos.unrealizedFunding ?? 0);
      continue;
    }

    const price = Number(pairPrices[pid]?.currentPrice ?? 0);
    if (price <= 0) continue;

    const posSize = Number(pos.size);
    const pairMmr = Number(pairParams[pid]?.maintenanceMarginRatio ?? 0);

    otherPnl += posSize * (price - Number(pos.entryPrice));
    otherMm += Math.abs(posSize) * price * pairMmr;
    totalFunding += Number(pos.unrealizedFunding ?? 0);
  }

  // C = margin + otherPnl - size * entryPrice - totalFunding - otherMm
  const c = margin + otherPnl - size * entryPrice - totalFunding - otherMm;

  // denom = |size| * mmr - size
  const denom = Math.abs(size) * mmr - size;
  if (denom === 0) return null;

  const liqPrice = c / denom;
  return liqPrice > 0 ? liqPrice : null;
}
