import { useCallback } from "react";

type UseTPSLPriceSyncParams = {
  setValue: (name: string, value: string) => void;
  referencePrice: number;
  isBuyDirection: boolean;
  enabled?: boolean;
};

/**
 * Returns onChange handlers that keep TP/SL price <-> percent fields in sync.
 *
 * The field the user is editing is written verbatim (so the input never gets
 * reformatted while typing); only the *other* field of the pair is recomputed.
 */
export function useTPSLPriceSync({
  setValue,
  referencePrice,
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

  const pctFromPrice = (price: number, isTakeProfit: boolean) => {
    const isUpside = isTakeProfit ? isBuyDirection : !isBuyDirection;
    const pct = isUpside
      ? ((price - referencePrice) / referencePrice) * 100
      : ((referencePrice - price) / referencePrice) * 100;
    return trim(Math.max(0, pct), 2);
  };

  const priceFromPct = (pct: number, isTakeProfit: boolean) => {
    const isUpside = isTakeProfit ? isBuyDirection : !isBuyDirection;
    const computed = isUpside
      ? referencePrice * (1 + pct / 100)
      : referencePrice * (1 - pct / 100);
    return trim(computed, 4);
  };

  const onTpPriceChange = useCallback(
    (value: string) => {
      setValue("tpPrice", value);
      if (!canCompute) return;
      const tp = Number(value);
      setValue("tpPercent", tp > 0 ? pctFromPrice(tp, true) : "");
    },
    [setValue, canCompute, referencePrice, isBuyDirection],
  );

  const onTpPercentChange = useCallback(
    (value: string) => {
      setValue("tpPercent", value);
      if (!canCompute) return;
      const pct = Number(value);
      setValue("tpPrice", pct > 0 ? priceFromPct(pct, true) : "");
    },
    [setValue, canCompute, referencePrice, isBuyDirection],
  );

  const onSlPriceChange = useCallback(
    (value: string) => {
      setValue("slPrice", value);
      if (!canCompute) return;
      const sl = Number(value);
      setValue("slPercent", sl > 0 ? pctFromPrice(sl, false) : "");
    },
    [setValue, canCompute, referencePrice, isBuyDirection],
  );

  const onSlPercentChange = useCallback(
    (value: string) => {
      setValue("slPercent", value);
      if (!canCompute) return;
      const pct = Number(value);
      setValue("slPrice", pct > 0 ? priceFromPct(pct, false) : "");
    },
    [setValue, canCompute, referencePrice, isBuyDirection],
  );

  return { onTpPriceChange, onTpPercentChange, onSlPriceChange, onSlPercentChange };
}
