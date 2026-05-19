import { useMemo } from "react";
import { useConfig } from "./useConfig.js";
import { perpsUserStateStore, perpsMarginAsset } from "./usePerpsUserState.js";

import type { AnyCoin, WithAmount } from "../types/coin.js";
import { TradePairStore } from "../stores/tradePairStore.js";

export function useTradeCoins() {
  const { coins } = useConfig();

  const userState = perpsUserStateStore((s) => s.userState);
  const pairId = TradePairStore((s) => s.pairId);

  if (!pairId) throw new Error("[useTradeCoins] pairId is required");

  const baseCoin: WithAmount<AnyCoin> = useMemo(() => {
    const base = coins.byDenom[pairId.baseDenom];
    if (!base) throw new Error(`[useTradeCoins] Base coin not found for denom ${pairId.baseDenom}`);
    return Object.assign({}, base, { amount: "0" });
  }, [pairId.baseDenom, coins]);

  const quoteCoin: WithAmount<AnyCoin> = useMemo(() => {
    const margin = userState?.margin ?? "0";
    return Object.assign(
      {},
      {
        symbol: perpsMarginAsset.symbol,
        denom: "usd",
        decimals: perpsMarginAsset.decimals,
        name: perpsMarginAsset.name,
        logoURI: perpsMarginAsset.logoURI,
        type: "native" as const,
      },
      { amount: margin },
    );
  }, [userState]);

  return { baseCoin, quoteCoin };
}
