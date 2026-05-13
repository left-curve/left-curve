import { useMemo } from "react";
import { useConfig } from "./useConfig.js";
import { useBalances } from "./useBalances.js";
import { useAccount } from "./useAccount.js";
import { perpsUserStateStore, perpsMarginAsset } from "./usePerpsUserState.js";

import { formatUnits } from "@left-curve/dango/utils";

import type { AnyCoin, WithAmount } from "../types/coin.js";
import { TradePairStore } from "../stores/tradePairStore.js";

export function useTradeCoins() {
  const { coins } = useConfig();
  const { account } = useAccount();
  const { data: balances = {} } = useBalances({ address: account?.address });

  const userState = perpsUserStateStore((s) => s.userState);
  const pairId = TradePairStore((s) => s.pairId);
  const mode = TradePairStore((s) => s.mode);

  if (!pairId) throw new Error("[useTradeCoins] pairId is required");

  const baseCoin: WithAmount<AnyCoin> = useMemo(() => {
    const base = coins.byDenom[pairId.baseDenom];

    if (!base) throw new Error(`[useTradeCoins] Base coin not found for denom ${pairId.baseDenom}`);

    const baseBalance = balances[pairId.baseDenom];

    const amount = mode === "spot" && baseBalance ? formatUnits(baseBalance, base.decimals) : "0";

    return Object.assign({}, base, { amount });
  }, [pairId.baseDenom, coins, balances, mode]);

  const quoteCoin: WithAmount<AnyCoin> = useMemo(() => {
    if (mode === "perps") {
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
    }

    const quote = coins.byDenom[pairId.quoteDenom];

    if (!quote)
      throw new Error(`[useTradeCoins] Quote coin not found for denom ${pairId.quoteDenom}`);

    const quoteBalance = balances[pairId.quoteDenom];

    const amount = quoteBalance ? formatUnits(quoteBalance || "0", quote.decimals) : "0";
    return Object.assign({}, quote, { amount });
  }, [pairId.quoteDenom, coins, balances, mode, userState]);

  return { baseCoin, quoteCoin };
}
