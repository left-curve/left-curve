import { useEffect, useRef } from "react";

type UseTPSLPriceSyncParams = {
  setValue: (name: string, value: string) => void;
  tpPrice: string;
  tpPercent: string;
  slPrice: string;
  slPercent: string;
  referencePrice: number;
  isBuyDirection: boolean;
  enabled?: boolean;
};

export function useTPSLPriceSync({
  setValue,
  tpPrice,
  tpPercent,
  slPrice,
  slPercent,
  referencePrice,
  isBuyDirection,
  enabled = true,
}: UseTPSLPriceSyncParams) {
  const tpSyncingRef = useRef(false);
  const slSyncingRef = useRef(false);

  useEffect(() => {
    if (tpSyncingRef.current || !enabled || referencePrice <= 0) return;
    const pct = Number(tpPercent);
    if (pct > 0) {
      const computed = isBuyDirection
        ? referencePrice * (1 + pct / 100)
        : referencePrice * (1 - pct / 100);
      tpSyncingRef.current = true;
      setValue("tpPrice", computed.toFixed(4));
      requestAnimationFrame(() => {
        tpSyncingRef.current = false;
      });
    }
  }, [tpPercent]);

  useEffect(() => {
    if (tpSyncingRef.current || !enabled || referencePrice <= 0) return;
    const tp = Number(tpPrice);
    if (tp > 0) {
      const pct = isBuyDirection
        ? ((tp - referencePrice) / referencePrice) * 100
        : ((referencePrice - tp) / referencePrice) * 100;
      tpSyncingRef.current = true;
      setValue("tpPercent", Math.max(0, pct).toFixed(2));
      requestAnimationFrame(() => {
        tpSyncingRef.current = false;
      });
    }
  }, [tpPrice]);

  useEffect(() => {
    if (slSyncingRef.current || !enabled || referencePrice <= 0) return;
    const pct = Number(slPercent);
    if (pct > 0) {
      const computed = isBuyDirection
        ? referencePrice * (1 - pct / 100)
        : referencePrice * (1 + pct / 100);
      slSyncingRef.current = true;
      setValue("slPrice", computed.toFixed(4));
      requestAnimationFrame(() => {
        slSyncingRef.current = false;
      });
    }
  }, [slPercent]);

  useEffect(() => {
    if (slSyncingRef.current || !enabled || referencePrice <= 0) return;
    const sl = Number(slPrice);
    if (sl > 0) {
      const pct = isBuyDirection
        ? ((referencePrice - sl) / referencePrice) * 100
        : ((sl - referencePrice) / referencePrice) * 100;
      slSyncingRef.current = true;
      setValue("slPercent", Math.max(0, pct).toFixed(2));
      requestAnimationFrame(() => {
        slSyncingRef.current = false;
      });
    }
  }, [slPrice]);
}
