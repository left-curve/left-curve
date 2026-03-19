import { useMemo } from "react";
import { useConfig } from "./useConfig.js";
import { useBalances } from "./useBalances.js";
import { useAccount } from "./useAccount.js";
import { perpsUserStateStore, perpsMarginAsset } from "./usePerpsUserState.js";

import { formatUnits } from "@left-curve/dango/utils";

import type { PairId } from "@left-curve/dango/types";
import type { AnyCoin, WithAmount } from "../types/coin.js";

type UseTradeCoinsParameters = {
  pairId: PairId;
  mode: "spot" | "perps";
};

export function useTradeCoins(parameters: UseTradeCoinsParameters) {
  const { pairId, mode } = parameters;
  const { coins } = useConfig();
  const { account } = useAccount();

  const { data: balances = {} } = useBalances({ address: account?.address });
  const userState = perpsUserStateStore((s) => s.userState);

  const baseCoin: WithAmount<AnyCoin> = useMemo(() => {
    const coin = coins.byDenom[pairId.baseDenom] ?? {
      symbol: pairId.baseDenom,
      denom: pairId.baseDenom,
      decimals: 6,
      name: pairId.baseDenom,
      type: "native" as const,
    };

    console.log(balances);

    const amount =
      mode === "spot" && balances[pairId.baseDenom]
        ? formatUnits(balances[pairId.baseDenom] || "0", coin.decimals)
        : "0";

    return Object.assign({}, coin, { amount });
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

    const coin = coins.byDenom[pairId.quoteDenom];
    const amount = balances[pairId.baseDenom]
      ? formatUnits(balances[pairId.quoteDenom] || "0", coin.decimals)
      : "0";
    return Object.assign({}, coin, { amount });
  }, [pairId.quoteDenom, coins, balances, mode, userState]);

  return { baseCoin, quoteCoin };
}
