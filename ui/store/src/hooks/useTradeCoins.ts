import { useMemo } from "react";
import { useConfig } from "./useConfig.js";
import { perpsMarginAsset, usePerpsUserState } from "./usePerpsUserState.js";

import type { AnyCoin, WithAmount } from "../types/coin.js";
import type { PairId } from "@left-curve/types";
import type { Config } from "../types/store.js";

export type UseTradePairCoinsParameters = {
  pairId: PairId;
};

export type UseTradeAccountCoinsParameters = UseTradePairCoinsParameters & {
  accountAddress?: string;
  enabled?: boolean;
};

export function getPerpsPairIdFromPairId(pairId: PairId, coins: Config["coins"]) {
  const base = coins.byDenom[pairId.baseDenom];
  if (!base) return "";
  return `perp/${base.symbol.toLowerCase()}usd`;
}

export function useTradePairCoins(parameters: UseTradePairCoinsParameters) {
  const { pairId } = parameters;
  const { coins } = useConfig();

  const baseCoin: WithAmount<AnyCoin> = useMemo(() => {
    const base = coins.byDenom[pairId.baseDenom];
    if (!base)
      throw new Error(`[useTradePairCoins] Base coin not found for denom ${pairId.baseDenom}`);
    return Object.assign({}, base, { amount: "0" });
  }, [pairId.baseDenom, coins]);

  const quoteCoin: WithAmount<AnyCoin> = useMemo(
    () => ({
      symbol: perpsMarginAsset.symbol,
      denom: "usd",
      decimals: perpsMarginAsset.decimals,
      name: perpsMarginAsset.name,
      logoURI: perpsMarginAsset.logoURI,
      type: "native" as const,
      amount: "0",
    }),
    [],
  );

  return { baseCoin, quoteCoin };
}

export function useTradeAccountCoins(parameters: UseTradeAccountCoinsParameters) {
  const { pairId, accountAddress, enabled = true } = parameters;
  const { baseCoin, quoteCoin } = useTradePairCoins({ pairId });
  const margin = usePerpsUserState((state) => state.userState?.margin ?? "0", {
    accountAddress,
    enabled,
  });

  const accountQuoteCoin = useMemo(
    () => Object.assign({}, quoteCoin, { amount: margin }),
    [quoteCoin, margin],
  );

  return { baseCoin, quoteCoin: accountQuoteCoin };
}
