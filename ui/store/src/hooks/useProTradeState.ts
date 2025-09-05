import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useAccount } from "./useAccount.js";
import { useBalances } from "./useBalances.js";
import { useConfig } from "./useConfig.js";
import { usePrices } from "./usePrices.js";
import { usePublicClient } from "./usePublicClient.js";
import { useSigningClient } from "./useSigningClient.js";
import { useSubmitTx } from "./useSubmitTx.js";
import { useQueryWithPagination } from "./useQueryWithPagination.js";

import { Direction } from "@left-curve/dango/types";
import { Decimal, capitalize, formatUnits, parseUnits } from "@left-curve/dango/utils";

import type { PairId } from "@left-curve/dango/types";
import type { AnyCoin, WithAmount } from "../types/coin.js";

export type UseProTradeStateParameters = {
  action: "buy" | "sell";
  onChangeAction: (action: "buy" | "sell") => void;
  orderType: "limit" | "market";
  onChangeOrderType: (order_type: "limit" | "market") => void;
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
  controllers: {
    inputs: Record<string, { value: string }>;
    reset: () => void;
    setValue: (name: string, value: string) => void;
  };
};

export function useProTradeState(parameters: UseProTradeStateParameters) {
  const {
    controllers,
    pairId,
    onChangePairId,
    onChangeAction,
    action: initialAction,
    orderType,
    onChangeOrderType,
  } = parameters;
  const { inputs, setValue } = controllers;
  const { account } = useAccount();
  const { coins } = useConfig();
  const queryClient = useQueryClient();
  const publicClient = usePublicClient();
  const { data: signingClient } = useSigningClient();

  const { convertAmount, getPrice, isFetched } = usePrices();

  const [sizeCoin, setSizeCoin] = useState(coins.byDenom[pairId.quoteDenom]);
  const [operation, setOperation] = useState(orderType);
  const [action, setAction] = useState(initialAction);

  const { data: balances = {} } = useBalances({
    address: account?.address,
  });

  const changePairId = useCallback((pairId: PairId) => {
    onChangePairId(pairId);
    setSizeCoin(coins.byDenom[pairId.quoteDenom]);
    setValue("size", "");
  }, []);

  const changeAction = useCallback((action: "buy" | "sell") => {
    setAction(action);
    setValue("size", "");
  }, []);

  const changeSizeCoin = useCallback((denom: string) => {
    setSizeCoin(coins.byDenom[denom]);
    setValue("size", "");
  }, []);

  const baseCoin: WithAmount<AnyCoin> = useMemo(
    () =>
      Object.assign({}, coins.byDenom[pairId.baseDenom], {
        amount: formatUnits(
          balances[pairId.baseDenom] || "0",
          coins.byDenom[pairId.baseDenom].decimals,
        ),
      }),
    [balances, coins, pairId],
  );

  const quoteCoin: WithAmount<AnyCoin> = useMemo(
    () =>
      Object.assign({}, coins.byDenom[pairId.quoteDenom], {
        amount: formatUnits(
          balances[pairId.quoteDenom] || "0",
          coins.byDenom[pairId.quoteDenom].decimals,
        ),
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

  const history = useQueryWithPagination({
    enabled: !!account,
    queryKey: ["tradeHistory", account?.address as string],
    queryFn: async () => {
      if (!account) throw new Error();
      return await publicClient.queryTrades({ address: account.address });
    },
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
      baseAmount: isBaseSize ? sizeValue : Decimal(sizeValue).divFloor(priceValue).toFixed(),
      quoteAmount: isQuoteSize ? sizeValue : Decimal(sizeValue).mul(priceValue).toFixed(),
    };
  }, [operation, sizeCoin, pairId, sizeValue, priceValue, needsConversion]);

  useEffect(() => {
    setValue("price", getPrice(1, pairId.baseDenom).toFixed(4));
  }, [isFetched, pairId]);

  useEffect(() => {
    onChangeOrderType(operation);
  }, [operation]);

  useEffect(() => {
    onChangeAction(action);
  }, [action]);

  const submission = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("No signing client available");
        if (!account) throw new Error("No account found");

        const direction = Direction[capitalize(action) as keyof typeof Direction];
        const { baseDenom, quoteDenom } = pairId;

        const limitAmount = Decimal(orderAmount.baseAmount)
          .times(Decimal(10).pow(baseCoin.decimals))
          .toFixed(0, 0);

        const price = Decimal(priceValue)
          .times(Decimal(10).pow(quoteCoin.decimals - baseCoin.decimals))
          .toFixed();

        const amount = (() => {
          if (operation === "market") {
            return (
              baseCoin.denom === availableCoin.denom
                ? parseUnits(orderAmount.baseAmount, baseCoin.decimals)
                : parseUnits(orderAmount.quoteAmount, quoteCoin.decimals)
            ).toString();
          }

          if (baseCoin.denom === availableCoin.denom)
            return parseUnits(orderAmount.baseAmount, baseCoin.decimals).toString();

          return Decimal(limitAmount).mulCeil(price).toFixed(0, 3);
        })();

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
                    amount: limitAmount,
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
          funds: {
            [availableCoin.denom]:
              operation === "market" ? Decimal(amount).mulCeil(1.18).toFixed() : amount,
          },
        });
      },
      onSuccess: () => {
        orders.refetch();
        history.refetch();
        controllers.reset();
        queryClient.invalidateQueries({ queryKey: ["quests", account?.username] });
        setValue("price", getPrice(1, pairId.baseDenom).toFixed(4));
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
    history,
    orders: {
      ...orders,
      data: orders.data ? orders.data : [],
    },
    submission,
    type: "spot",
  };
}
