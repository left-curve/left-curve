import { Decimal } from "@left-curve/sdk/utils";

import type { VaultSnapshot } from "../types/perps.js";

const VIRTUAL_SHARES = "1000000";
const VIRTUAL_ASSETS = "1";

/**
 * Convert vault shares to their USD value using the virtual-offset formula.
 * Mirrors the on-chain calculation in `dango/perps/src/vault/`.
 */
export function sharesToUsd(shares: string, vaultEquity: string, shareSupply: string): string {
  if (shares === "0") return "0";
  const effectiveSupply = Decimal(shareSupply).plus(VIRTUAL_SHARES);
  const effectiveEquity = Decimal(vaultEquity).plus(VIRTUAL_ASSETS);
  if (effectiveSupply.isZero()) return "0";
  return Decimal(shares).mul(effectiveEquity).div(effectiveSupply).toString();
}

/**
 * Convert a USD deposit amount to the number of vault shares received.
 * Inverse of `sharesToUsd`.
 */
export function usdToShares(usdAmount: string, vaultEquity: string, shareSupply: string): string {
  if (usdAmount === "0") return "0";
  const effectiveSupply = Decimal(shareSupply).plus(VIRTUAL_SHARES);
  const effectiveEquity = Decimal(vaultEquity).plus(VIRTUAL_ASSETS);
  if (effectiveEquity.isZero()) return "0";
  return Decimal(usdAmount).mul(effectiveSupply).div(effectiveEquity).toFixed(0);
}

export function computeVaultApy(snapshots: Record<string, VaultSnapshot>): string | null {
  const entries = Object.entries(snapshots).sort(([a], [b]) => Number(a) - Number(b));
  if (entries.length < 2) return null;

  const [firstTs, first] = entries[0];
  const [lastTs, last] = entries[entries.length - 1];

  const startPrice = Decimal(first.equity).div(first.shareSupply);
  const endPrice = Decimal(last.equity).div(last.shareSupply);

  if (startPrice.isZero()) return null;

  const days = Decimal(Number(lastTs) - Number(firstTs)).div(86_400);
  if (days.isZero()) return null;

  const priceRatio = endPrice.div(startPrice);
  const exponent = Decimal(365).div(days);
  const apy = Decimal(priceRatio.toNumber() ** exponent.toNumber() - 1)
    .mul(100)
    .toFixed(2);

  return apy;
}
