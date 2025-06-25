import { useQuery } from "@tanstack/react-query";
import { useCallback, useMemo, useState } from "react";
import { useAccount } from "./useAccount.js";
import { useBalances } from "./useBalances.js";
import { useConfig } from "./useConfig.js";
import { usePrices } from "./usePrices.js";
import { usePublicClient } from "./usePublicClient.js";
import { useSigningClient } from "./useSigningClient.js";
import { useSubmitTx } from "./useSubmitTx.js";

import { Direction } from "@left-curve/dango/types";
import { capitalize, formatUnits, parseUnits } from "@left-curve/dango/utils";

import type { PairId } from "@left-curve/dango/types";
import type { AnyCoin, WithAmount } from "../types/coin.js";

export type UseProTradeStateParameters = {
  action: "buy" | "sell";
  onChangeAction: (action: "buy" | "sell") => void;
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
  controllers: {
    inputs: Record<string, { value: string }>;
    reset: () => void;
    setValue: (name: string, value: string) => void;
  };
};

export function useProTradeState(parameters: UseProTradeStateParameters) {
  const { controllers, pairId, onChangePairId, action, onChangeAction } = parameters;
  const { inputs, setValue } = controllers;
  const { account } = useAccount();
  const { coins } = useConfig();
  const publicClient = usePublicClient();
  const { data: signingClient } = useSigningClient();

  const { convertAmount } = usePrices();

  const [sizeCoin, setSizeCoin] = useState(coins[pairId.quoteDenom]);
  const [operation, setOperation] = useState<"market" | "limit">("market");

  const { data: balances = {}, refetch: updateBalance } = useBalances({
    address: account?.address,
  });

  const changeAction = useCallback((action: "buy" | "sell") => {
    onChangeAction(action);
    setValue("size", "0");
  }, []);

  const changeSizeCoin = useCallback((denom: string) => {
    setSizeCoin(coins[denom]);
    setValue("size", "0");
  }, []);

  const baseCoin: WithAmount<AnyCoin> = useMemo(
    () =>
      Object.assign({}, coins[pairId.baseDenom], {
        amount: formatUnits(balances[pairId.baseDenom] || "0", coins[pairId.baseDenom].decimals),
      }),
    [balances, coins, pairId],
  );

  const quoteCoin: WithAmount<AnyCoin> = useMemo(
    () =>
      Object.assign({}, coins[pairId.quoteDenom], {
        amount: formatUnits(balances[pairId.quoteDenom] || "0", coins[pairId.quoteDenom].decimals),
      }),
    [balances, coins, pairId],
  );

  const availableCoin = action === "buy" ? quoteCoin : baseCoin;

  const needsConversion = sizeCoin.denom !== availableCoin.denom;

  const maxSizeAmount = useMemo(() => {
    if (availableCoin.amount === "0") return 0;
    return needsConversion
      ? convertAmount(availableCoin.amount, availableCoin.denom, sizeCoin.denom)
      : +availableCoin.amount;
  }, [sizeCoin, availableCoin, needsConversion]);

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

  const submission = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("No signing client available");
        if (!account) throw new Error("No account found");

        const direction = Direction[capitalize(action) as keyof typeof Direction];
        const { baseDenom, quoteDenom } = pairId;

        const amount = needsConversion
          ? convertAmount(inputs.size.value, sizeCoin.denom, availableCoin.denom, true)
          : parseUnits(inputs.size.value, availableCoin.decimals).toString();

        const order =
          operation === "market"
            ? {
                createsMarket: [
                  {
                    baseDenom,
                    quoteDenom,
                    amount,
                    direction,
                    maxSlippage: "0.08",
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
                    price: parseUnits(inputs.price.value, quoteCoin.decimals).toString(),
                  },
                ],
              };

        await signingClient.batchUpdateOrders({
          sender: account.address,
          ...order,
          funds: { [availableCoin.denom]: amount },
        });

        await orders.refetch();
        await updateBalance();
        controllers.reset();
      },
    },
  });

  return {
    pairId,
    onChangePairId,
    maxSizeAmount,
    availableCoin,
    baseCoin,
    quoteCoin,
    sizeCoin,
    changeSizeCoin,
    operation,
    setOperation,
    action,
    changeAction,
    orders,
    submission,
    type: "spot",
  };
}
