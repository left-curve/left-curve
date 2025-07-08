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
import { Decimal, capitalize, formatUnits, parseUnits } from "@left-curve/dango/utils";

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
  const { controllers, pairId, onChangePairId, onChangeAction, action } = parameters;
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

  const changePairId = useCallback((pairId: PairId) => {
    onChangePairId(pairId);
    setSizeCoin(coins[pairId.quoteDenom]);
    setValue("size", "0");
  }, []);

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

  const sizeValue = inputs.size?.value || "0";
  const priceValue = inputs.price?.value || "0";

  const maxSizeAmount = useMemo(() => {
    if (availableCoin.amount === "0") return 0;
    if (!needsConversion) return +availableCoin.amount;

    return operation === "limit"
      ? (() => {
          if (priceValue === "0") return 0;
          return action === "buy"
            ? Decimal(availableCoin.amount).div(priceValue).toNumber()
            : Decimal(availableCoin.amount).mul(priceValue).toNumber();
        })()
      : convertAmount(availableCoin.amount, availableCoin.denom, sizeCoin.denom);
  }, [sizeCoin, availableCoin, needsConversion, priceValue]);

  const orders = useQuery({
    enabled: !!account,
    queryKey: ["ordersByUser", account?.address],
    queryFn: async () => {
      if (!account) return [];
      const response = await publicClient.ordersByUser({ user: account.address });
      return Object.entries(response).map(([id, order]) => ({
        ...order,
        id,
      }));
    },
    initialData: [],
    refetchInterval: 1000 * 10,
  });

  const orderAmount = useMemo(() => {
    if (sizeValue === "0") return { baseAmount: "0", quoteAmount: "0" };

    const isBaseSize = sizeCoin.denom === pairId.baseDenom;
    const isQuoteSize = sizeCoin.denom === pairId.quoteDenom;

    if (operation === "market") {
      return {
        baseAmount: isBaseSize
          ? sizeValue
          : convertAmount(sizeValue, sizeCoin.denom, pairId.baseDenom).toString(),
        quoteAmount: isQuoteSize
          ? sizeValue
          : convertAmount(sizeValue, sizeCoin.denom, pairId.quoteDenom).toString(),
      };
    }

    if (priceValue === "0") return { baseAmount: "0", quoteAmount: "0" };

    return {
      baseAmount: isBaseSize ? sizeValue : Decimal(sizeValue).divFloor(priceValue).toString(),
      quoteAmount: isQuoteSize ? sizeValue : Decimal(sizeValue).mul(priceValue).toString(),
    };
  }, [operation, sizeCoin, pairId, sizeValue, priceValue, needsConversion]);

  const submission = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("No signing client available");
        if (!account) throw new Error("No account found");

        const direction = Direction[capitalize(action) as keyof typeof Direction];
        const { baseDenom, quoteDenom } = pairId;

        const amount = (
          baseCoin.denom === availableCoin.denom
            ? parseUnits(orderAmount.baseAmount, baseCoin.decimals)
            : parseUnits(orderAmount.quoteAmount, quoteCoin.decimals)
        ).toString();

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
                    amount: parseUnits(orderAmount.baseAmount, baseCoin.decimals).toString(),
                    baseDenom,
                    quoteDenom,
                    direction,
                    price: Decimal(inputs.price.value)
                      .times(Decimal(10).pow(quoteCoin.decimals - baseCoin.decimals))
                      .toFixed(),
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
    onChangePairId: changePairId,
    orderAmount,
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
    orders: {
      ...orders,
      data: orders.data ? orders.data : [],
    },
    submission,
    type: "spot",
  };
}
