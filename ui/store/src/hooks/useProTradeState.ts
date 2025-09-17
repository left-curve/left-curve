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
import { useAppConfig } from "./useAppConfig.js";
import { useOrderBookState } from "./useOrderBookState.js";
import { useLiquidityDepthState } from "./useLiquidityDepthState.js";

import { Decimal, formatUnits, parseUnits } from "@left-curve/dango/utils";

import type { CreateOrderRequest, PairId, PriceOption } from "@left-curve/dango/types";
import type { AnyCoin, WithAmount } from "../types/coin.js";
import { useLiveTradesState } from "./useLiveTradesState.js";

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
  const queryClient = useQueryClient();
  const publicClient = usePublicClient();

  const { inputs, setValue } = controllers;
  const { account } = useAccount();
  const { coins } = useConfig();
  const { data: appConfig } = useAppConfig();
  const { data: signingClient } = useSigningClient();

  const { convertAmount, getPrice, isFetched } = usePrices();

  const [sizeCoin, setSizeCoin] = useState(coins.byDenom[pairId.quoteDenom]);
  const [operation, setOperation] = useState(orderType);
  const [action, setAction] = useState(initialAction);

  const { data: balances = {} } = useBalances({
    address: account?.address,
  });

  const pair = appConfig?.pairs[pairId.baseDenom]!;

  const [bucketSize, setBucketSize] = useState(pair.params.bucketSizes[0]);

  const { liquidityDepthStore } = useLiquidityDepthState({
    subscribe: true,
    pairId,
    bucketSize,
  });

  const { orderBookStore } = useOrderBookState({ pairId, subscribe: true });
  const orderBookState = orderBookStore((s) => s.orderBook);

  const { liveTradesStore } = useLiveTradesState({ pairId, subscribe: true });

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

  const amount = useMemo(() => {
    if (!orderBookState) return { base: "0", quote: "0" };
    if (sizeValue === "0") return { base: "0", quote: "0" };

    const isBaseSize = sizeCoin.denom === pairId.baseDenom;
    const isQuoteSize = sizeCoin.denom === pairId.quoteDenom;

    const price = parseUnits(
      operation === "market" ? orderBookState.midPrice || "0" : priceValue || "0",
      baseCoin.decimals - quoteCoin.decimals,
    );

    return {
      base: isBaseSize ? sizeValue : Decimal(sizeValue).divFloor(price).toFixed(),
      quote: isQuoteSize ? sizeValue : Decimal(sizeValue).mulCeil(price).toFixed(),
    };
  }, [orderBookState, operation, sizeCoin, pairId, sizeValue, priceValue]);

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

        const { baseDenom, quoteDenom } = pairId;

        const parsedAmount =
          baseCoin.denom === availableCoin.denom
            ? parseUnits(amount.base, baseCoin.decimals)
            : parseUnits(amount.quote, quoteCoin.decimals);

        const price: PriceOption =
          operation === "market"
            ? { market: { maxSlippage: "0.001" } }
            : { limit: parseUnits(priceValue, baseCoin.decimals - quoteCoin.decimals) };

        const order: CreateOrderRequest = {
          baseDenom,
          quoteDenom,
          price,
          amount:
            action === "buy" ? { bid: { quote: parsedAmount } } : { ask: { base: parsedAmount } },
          timeInForce: operation === "market" ? "IOC" : "GTC",
        };

        await signingClient.batchUpdateOrders({
          sender: account.address,
          creates: [order],
          funds: {
            [availableCoin.denom]: parsedAmount,
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
    bucketSize,
    setBucketSize,
    pair,
    pairId,
    onChangePairId: changePairId,
    amount,
    maxSizeAmount,
    availableCoin,
    orderBookStore,
    liveTradesStore,
    liquidityDepthStore,
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
