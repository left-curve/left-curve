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
  lastUpdatedBlockHeight: number;
  orderBook: RestingOrderBookState | null;
  previousPrice: string;
  currentPrice: string;
  setState: ({
    orderBook,
    currentPrice,
    blockHeight,
  }: Omit<OrderBookStoreState, "setState" | "previousPrice" | "lastUpdatedBlockHeight"> & {
    blockHeight: number;
  }) => void;
};

export const orderBookStore = create<OrderBookStoreState>((set, get) => ({
  lastUpdatedBlockHeight: 0,
  orderBook: null,
  currentPrice: "0",
  previousPrice: "0",
  setState: ({ orderBook, currentPrice, blockHeight }) => {
    const { currentPrice: previousPrice, lastUpdatedBlockHeight } = get();
    if (blockHeight <= lastUpdatedBlockHeight) return;
    set(() => ({ orderBook, previousPrice, currentPrice, lastUpdatedBlockHeight: blockHeight }));
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
        type Event = {
          response: { wasmSmart: RestingOrderBookState };
          blockHeight: number;
        };

        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        const { wasmSmart: orderBook } = response;
        const currentPrice = parseUnits(
          orderBook.midPrice as string,
          coins.byDenom[pairId.baseDenom].decimals - coins.byDenom[pairId.quoteDenom].decimals,
        );

        setState({ orderBook, currentPrice, blockHeight });
      },
    });

    return () => {
      unsubscribe();
    };
  }, [pairId, subscribe, appConfig]);

  return { orderBookStore };
}
