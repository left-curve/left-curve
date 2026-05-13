import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createBlockStore } from "./createBlockStore.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { Denom, Price, QueryRequest } from "@left-curve/dango/types";

type UseOraclePricesParameters = {
  subscribe?: boolean;
  interval?: number;
};

export const oraclePricesStore = createBlockStore({
  initialState: {
    prices: {} as Record<Denom, Price>,
  },
});

export function useOraclePrices(parameters?: UseOraclePricesParameters) {
  const { subscribe, interval = 1 } = parameters || {};
  const { subscriptions } = useConfig();
  const { data: appConfig } = useAppConfig();

  const { setState } = oraclePricesStore();

  useEffect(() => {
    if (!subscribe) return;

    const { addresses } = appConfig;
    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval,
        httpInterval: 2_000,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.oracle,
            msg: {
              prices: {},
            },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: Record<Denom, Price> };
          blockHeight: number;
        };

        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        const { wasmSmart: prices } = response;

        setState({ prices, blockHeight });
      },
    });

    return () => {
      unsubscribe();
    };
  }, [subscribe, interval, appConfig.addresses]);

  return { oraclePricesStore };
}
