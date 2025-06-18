import { useQuery } from "@tanstack/react-query";
import { useCallback, useState } from "react";
import { useAccount } from "./useAccount.js";
import { usePublicClient } from "./usePublicClient.js";

import type { PairId } from "@left-curve/dango/types";
import { useBalances } from "./useBalances.js";
import { useConfig } from "./useConfig.js";

export type UseProTradeParameters = {
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
  inputs: Record<string, { value: string }>;
};

export function useProTrade(parameters: UseProTradeParameters) {
  const { inputs, pairId, onChangePairId } = parameters;
  const { account } = useAccount();
  const { coins } = useConfig();
  const publicClient = usePublicClient();

  const baseCoin = coins[pairId.baseDenom];
  const quoteCoin = coins[pairId.quoteDenom];

  const [coin, setCoin] = useState(baseCoin);
  const [operation, setOperation] = useState<"market" | "limit">("limit");
  const [action, setAction] = useState<"buy" | "sell">("buy");

  const { data: balances = {} } = useBalances({ address: account?.address });

  const balance = balances[coin.denom] || "0";

  const onChangeCoin = useCallback((denom: string) => setCoin(coins[denom]), [pairId]);

  const orders = useQuery({
    enabled: !!account,
    queryKey: ["ordersByUser", account?.address],
    queryFn: async () => {
      if (!account) return [];
      const response = await publicClient.ordersByUser({ user: account.address });
      return Object.entries(response).map(([id, order]) => ({
        ...order,
        id: +id,
      }));
    },
    initialData: [],
    refetchInterval: 1000 * 10,
  });

  return {
    pairId,
    coin: Object.assign({}, coin, { balance }),
    onChangeCoin,
    coins: [baseCoin, quoteCoin],
    balance,
    onChangePairId,
    orders,
    operation,
    setOperation,
    action,
    setAction,
    type: "spot",
  };
}
