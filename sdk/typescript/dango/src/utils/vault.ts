import { Decimal } from "@left-curve/sdk/utils";

const VIRTUAL_SHARES = "1000000";
const VIRTUAL_ASSETS = "1";

/**
 * Convert vault shares to their USD value using the virtual-offset formula.
 * Mirrors the on-chain calculation in `dango/perps/src/vault/`.
 */
export function sharesToUsd(
  shares: string,
  vaultEquity: string,
  shareSupply: string,
): string {
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
export function usdToShares(
  usdAmount: string,
  vaultEquity: string,
  shareSupply: string,
): string {
  if (usdAmount === "0") return "0";
  const effectiveSupply = Decimal(shareSupply).plus(VIRTUAL_SHARES);
  const effectiveEquity = Decimal(vaultEquity).plus(VIRTUAL_ASSETS);
  if (effectiveEquity.isZero()) return "0";
  return Decimal(usdAmount).mul(effectiveSupply).div(effectiveEquity).toFixed(0);
}
