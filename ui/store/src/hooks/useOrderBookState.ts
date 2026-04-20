import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createBlockStore } from "./createBlockStore.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { QueryRequest, RestingOrderBookState } from "@left-curve/dango/types";
import { parseUnits } from "@left-curve/dango/utils";
import { TradePairStore } from "../index.js";

type UseOrderBookStateParameters = {
  subscribe?: boolean;
};

export const orderBookStore = createBlockStore({
  initialState: {
    orderBook: null as RestingOrderBookState | null,
    currentPrice: "0",
    previousPrice: "0",
  },
  beforeUpdate: (prev) => ({ previousPrice: prev.currentPrice }),
});

export function useOrderBookState(parameters?: UseOrderBookStateParameters) {
  const { subscribe } = parameters || {};
  const { subscriptions, coins } = useConfig();
  const { data: appConfig } = useAppConfig();

  const pairId = TradePairStore((s) => s.pairId);

  const { setState } = orderBookStore();

  useEffect(() => {
    if (!subscribe || !pairId) return;

    const { addresses } = appConfig;
    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 1,
        httpInterval: 2_000,
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
  }, [pairId, subscribe, appConfig.addresses]);

  return { orderBookStore };
}
