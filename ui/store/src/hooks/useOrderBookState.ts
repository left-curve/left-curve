import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { PairId, QueryRequest, RestingOrderBookState } from "@left-curve/dango/types";
import { parseUnits } from "@left-curve/dango/utils";
import { create } from "zustand";

type UseOrderBookStateParameters = {
  pairId: PairId;
  subscribe?: boolean;
};

export type OrderBookStoreState = {
  orderBook: RestingOrderBookState | null;
  previousPrice: string;
  currentPrice: string;
  setState: ({
    orderBook,
    currentPrice,
  }: Omit<OrderBookStoreState, "setState" | "previousPrice">) => void;
};

const orderBookStore = create<OrderBookStoreState>((set, get) => ({
  orderBook: null,
  currentPrice: "0",
  previousPrice: "0",
  setState: ({ orderBook, currentPrice }) => {
    const { currentPrice: previousPrice } = get();
    set(() => ({ orderBook, previousPrice, currentPrice }));
  },
}));

export function useOrderBookState(parameters: UseOrderBookStateParameters) {
  const { pairId, subscribe } = parameters;
  const { subscriptions, coins } = useConfig();
  const { data: appConfig } = useAppConfig();

  const { setState } = orderBookStore();

  useEffect(() => {
    if (!appConfig || !subscribe) return;

    const { addresses } = appConfig;
    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 1,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.dex,
            msg: {
              restingOrderBookState: {
                baseDenom: pairId.baseDenom,
                quoteDenom: pairId.quoteDenom,
              },
            },
          },
        }),
      },
      listener: (event) => {
        type Event = { wasmSmart: RestingOrderBookState };
        const { wasmSmart: orderBook } = camelCaseJsonDeserialization<Event>(event);

        const currentPrice = parseUnits(
          orderBook.midPrice as string,
          coins.byDenom[pairId.baseDenom].decimals - coins.byDenom[pairId.quoteDenom].decimals,
        );

        setState({ orderBook, currentPrice });
      },
    });

    return () => {
      unsubscribe();
    };
  }, [pairId, subscribe, appConfig]);

  return { orderBookStore };
}
