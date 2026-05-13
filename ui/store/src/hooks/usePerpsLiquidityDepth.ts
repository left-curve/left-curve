import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createBlockStore } from "./createBlockStore.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { PerpsLiquidityDepthResponse, QueryRequest } from "@left-curve/dango/types";

type UsePerpsLiquidityDepthParameters = {
  pairId: string;
  bucketSize: string;
  limit?: number;
  subscribe?: boolean;
};

export const perpsLiquidityDepthStore = createBlockStore({
  initialState: {
    liquidityDepth: null as PerpsLiquidityDepthResponse | null,
  },
});

export function usePerpsLiquidityDepth(parameters: UsePerpsLiquidityDepthParameters) {
  const { pairId, bucketSize, limit = 20, subscribe } = parameters;
  const { subscriptions } = useConfig();
  const { data: appConfig } = useAppConfig();

  const { setState } = perpsLiquidityDepthStore();

  useEffect(() => {
    if (!appConfig || !subscribe) return;

    const { addresses } = appConfig;
    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 1,
        httpInterval: 2_000,
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

        setState({ liquidityDepth, blockHeight });
      },
    });

    return () => {
      unsubscribe();
    };
  }, [pairId, bucketSize, limit, subscribe, appConfig]);

  return { perpsLiquidityDepthStore };
}
