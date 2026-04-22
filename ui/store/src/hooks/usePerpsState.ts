import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createBlockStore } from "./createBlockStore.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { PerpsState, QueryRequest } from "@left-curve/dango/types";

export const perpsStateStore = createBlockStore({
  initialState: { state: null as PerpsState | null },
});

type UsePerpsStateParameters = {
  subscribe?: boolean;
};

export function usePerpsState(parameters?: UsePerpsStateParameters) {
  const { subscribe = true } = parameters ?? {};
  const { subscriptions } = useConfig();
  const { data: appConfig } = useAppConfig();

  const { setState } = perpsStateStore();

  useEffect(() => {
    if (!subscribe) return;
    const { addresses } = appConfig;

    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 5,
        httpInterval: 5_000,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.perps,
            msg: { state: {} },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsState | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        setState({ state: response.wasmSmart, blockHeight });
      },
    });

    return () => unsubscribe();
  }, [appConfig.addresses, subscribe]);

  return { perpsStateStore };
}
