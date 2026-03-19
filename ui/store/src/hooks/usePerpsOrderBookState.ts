import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createBlockStore } from "./createBlockStore.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { PerpsLiquidityDepthResponse, QueryRequest } from "@left-curve/dango/types";

type UsePerpsOrderBookStateParameters = {
  pairId: string;
  bucketSize: string;
  limit?: number;
  subscribe?: boolean;
};

export const perpsOrderBookStore = createBlockStore({
  initialState: {
    liquidityDepth: null as PerpsLiquidityDepthResponse | null,
    currentPrice: "0",
    previousPrice: "0",
  },
  beforeUpdate: (prev) => ({ previousPrice: prev.currentPrice }),
});

export function usePerpsOrderBookState(parameters: UsePerpsOrderBookStateParameters) {
  const { pairId, bucketSize, limit = 20, subscribe } = parameters;
  const { subscriptions } = useConfig();
  const { data: appConfig } = useAppConfig();

  const { setState } = perpsOrderBookStore();

  useEffect(() => {
    if (!appConfig || !subscribe) return;

    const { addresses } = appConfig;
    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 1,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.perps,
            msg: {
              liquidityDepth: {
                pairId,
                bucketSize,
                limit,
              },
            },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsLiquidityDepthResponse };
          blockHeight: number;
        };

        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        const { wasmSmart: liquidityDepth } = response;

        const bidPrices = Object.keys(liquidityDepth.bids);
        const askPrices = Object.keys(liquidityDepth.asks);

        let currentPrice = "0";
        if (bidPrices.length > 0 && askPrices.length > 0) {
          const bestBid = bidPrices[bidPrices.length - 1];
          const bestAsk = askPrices[0];
          currentPrice = String((Number(bestBid) + Number(bestAsk)) / 2);
        } else if (bidPrices.length > 0) {
          currentPrice = bidPrices[bidPrices.length - 1];
        } else if (askPrices.length > 0) {
          currentPrice = askPrices[0];
        }

        setState({ liquidityDepth, currentPrice, blockHeight });
      },
    });

    return () => {
      unsubscribe();
    };
  }, [pairId, bucketSize, limit, subscribe, appConfig]);

  return { perpsOrderBookStore };
}
