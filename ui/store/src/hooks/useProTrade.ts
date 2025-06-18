import { useMutation, useQuery } from "@tanstack/react-query";
import { useCallback, useState } from "react";
import { useAccount } from "./useAccount.js";
import { useBalances } from "./useBalances.js";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";
import { useSigningClient } from "./useSigningClient.js";

import { Direction } from "@left-curve/dango/types";
import { capitalize, parseUnits } from "@left-curve/dango/utils";

import type { PairId } from "@left-curve/dango/types";

export type UseProTradeParameters = {
  action: "buy" | "sell";
  onChangeAction: (action: "buy" | "sell") => void;
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
  inputs: Record<string, { value: string }>;
};

export function useProTrade(parameters: UseProTradeParameters) {
  const { inputs, pairId, onChangePairId, action, onChangeAction } = parameters;
  const { account } = useAccount();
  const { coins } = useConfig();
  const publicClient = usePublicClient();
  const { data: signingClient } = useSigningClient();

  const baseCoin = coins[pairId.baseDenom];
  const quoteCoin = coins[pairId.quoteDenom];

  const [coin, setCoin] = useState(baseCoin);
  const [operation, setOperation] = useState<"market" | "limit">("market");

  const { data: balances = {}, refetch: updateBalance } = useBalances({
    address: account?.address,
  });

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

  const submission = useMutation({
    mutationFn: async () => {
      if (!signingClient) throw new Error("No signing client available");
      if (!account) throw new Error("No account found");

      const amount = parseUnits(inputs.size.value, coin.decimals).toString();
      const price = inputs.price.value;
      const direction = Direction[capitalize(action) as keyof typeof Direction];
      const { baseDenom, quoteDenom } = pairId;

      const order =
        operation === "market"
          ? {
              createsMarket: [
                {
                  baseDenom,
                  quoteDenom,
                  amount,
                  direction,
                  maxSlippage: "0.01",
                },
              ],
            }
          : {
              createsLimit: [
                {
                  amount,
                  baseDenom,
                  quoteDenom,
                  direction,
                  price,
                },
              ],
            };

      await signingClient.batchUpdateOrders({
        sender: account.address,
        ...order,
      });

      await orders.refetch();
      await updateBalance();
    },
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
    onChangeAction,
    submission,
    type: "spot",
  };
}
