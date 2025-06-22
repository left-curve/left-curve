import { useCallback, useEffect, useState } from "react";
import { useAccount } from "./useAccount.js";
import { useBalances } from "./useBalances.js";
import { useConfig } from "./useConfig.js";

import type { PairId } from "@left-curve/dango/types";

export type UsePoolLiquidityParameters = {
  action: "deposit" | "withdraw";
  onChangeAction: (action: "deposit" | "withdraw") => void;
  pairId: PairId;
};

export function usePoolLiquidity(parameters: UsePoolLiquidityParameters) {
  const { pairId, action, onChangeAction } = parameters;
  const { account } = useAccount();
  const { coins } = useConfig();
  const userLiquidity = false;

  const baseCoin = coins[pairId.baseDenom];
  const quoteCoin = coins[pairId.quoteDenom];

  const [coin, setCoin] = useState(baseCoin);

  const { data: balances = {}, refetch: updateBalance } = useBalances({
    address: account?.address,
  });

  const balance = balances[coin.denom] || "0";

  const onChangeCoin = useCallback((denom: string) => setCoin(coins[denom]), [pairId]);

  return {
    pairId,
    coin: Object.assign({}, coin, { balance }),
    onChangeCoin,
    coins: { baseCoin, quoteCoin },
    balance,
    action,
    onChangeAction,
    userLiquidity,
  };
}
