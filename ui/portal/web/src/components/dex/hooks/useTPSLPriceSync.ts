import { useCallback } from "react";

import { priceFromRoi, roiFromPrice, type TpslKind } from "../helpers/math";

type UseTPSLPriceSyncParams = {
  setValue: (name: string, value: string) => void;
  referencePrice: number;
  /**
   * Effective leverage on the position. The percent field is computed as
   * `price_delta_pct × leverage`, matching how Binance / Bybit / Hyperliquid /
   * dYdX display TP/SL ROI on capital. A missing or zero leverage degrades
   * gracefully to the "raw price delta" behavior (`clampLeverage` in
   * `./tpslMath`).
   */
  leverage: number;
  isBuyDirection: boolean;
  enabled?: boolean;
};

/**
 * Returns onChange handlers that keep TP/SL price ↔ percent fields in sync.
 *
 * The percent represents **ROI% on capital** (price delta × leverage), not
 * the raw price change. Actual math is delegated to `./tpslMath` so it can
 * be exhaustively unit-tested without a renderer.
 *
 * The field the user is editing is written verbatim (so the input never gets
 * reformatted while typing); only the *other* field of the pair is recomputed.
 *
 * Note: handlers do not react to later changes in `leverage` or
 * `referencePrice` — the percent label becomes stale until the user re-types.
 * This matches the pre-existing behavior of the hook w.r.t. `referencePrice`
 * and is a known limitation; a follow-up can add a leverage-change effect
 * once we pick whether to preserve the price or the percent.
 */
export function useTPSLPriceSync({
  setValue,
  referencePrice,
  leverage,
  isBuyDirection,
  enabled = true,
}: UseTPSLPriceSyncParams) {
  const canCompute = enabled && referencePrice > 0;

  // Format with up to N decimals, stripping trailing zeros so exact values
  // (e.g. 5, 100) display without padding like "5.00" / "100.0000".
  const trim = (n: number, maxDecimals: number) => {
    if (!Number.isFinite(n)) return "";
    const fixed = n.toFixed(maxDecimals);
    return fixed.includes(".") ? fixed.replace(/\.?0+$/, "") : fixed;
  };

  const pctField = (price: number, kind: TpslKind) => {
    const roi = roiFromPrice(price, {
      referencePrice,
      leverage,
      isLong: isBuyDirection,
      kind,
    });
    // Clamp to 0 for display: a negative ROI means a wrong-side trigger
    // (e.g. long TP below entry). Validation rejects the submission — we
    // just don't want to show a negative percent while the user is typing.
    return trim(Math.max(0, roi), 2);
  };

  const priceField = (roi: number, kind: TpslKind) => {
    const price = priceFromRoi(roi, {
      referencePrice,
      leverage,
      isLong: isBuyDirection,
      kind,
    });
    return trim(price, 4);
  };

  const onTpPriceChange = useCallback(
    (value: string) => {
      setValue("tpPrice", value);
      if (!canCompute) return;
      const tp = Number(value);
      setValue("tpPercent", tp > 0 ? pctField(tp, "tp") : "");
    },
    [setValue, canCompute, referencePrice, leverage, isBuyDirection],
  );

  const onTpPercentChange = useCallback(
    (value: string) => {
      setValue("tpPercent", value);
      if (!canCompute) return;
      const roi = Number(value);
      setValue("tpPrice", roi > 0 ? priceField(roi, "tp") : "");
    },
    [setValue, canCompute, referencePrice, leverage, isBuyDirection],
  );

  const onSlPriceChange = useCallback(
    (value: string) => {
      setValue("slPrice", value);
      if (!canCompute) return;
      const sl = Number(value);
      setValue("slPercent", sl > 0 ? pctField(sl, "sl") : "");
    },
    [setValue, canCompute, referencePrice, leverage, isBuyDirection],
  );

  const onSlPercentChange = useCallback(
    (value: string) => {
      setValue("slPercent", value);
      if (!canCompute) return;
      const roi = Number(value);
      setValue("slPrice", roi > 0 ? priceField(roi, "sl") : "");
    },
    [setValue, canCompute, referencePrice, leverage, isBuyDirection],
  );

  return { onTpPriceChange, onTpPercentChange, onSlPriceChange, onSlPercentChange };
}
