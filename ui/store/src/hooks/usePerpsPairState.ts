import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createBlockStore } from "./createBlockStore.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { PerpsPairState, QueryRequest } from "@left-curve/dango/types";
import { TradePairStore } from "../stores/tradePairStore.js";

export const perpsPairStateStore = createBlockStore({
  initialState: { pairState: null as PerpsPairState | null, pairId: null as string | null },
});

type UsePerpsPairStateParameters = {
  subscribe?: boolean;
};

export function usePerpsPairState(parameters: UsePerpsPairStateParameters) {
  const { subscribe = true } = parameters;
  const { subscriptions } = useConfig();
  const { data: appConfig } = useAppConfig();

  const pairId = TradePairStore((s) => s.pairId);
  const getPerpsPairId = TradePairStore((s) => s.getPerpsPairId);

  const { setState } = perpsPairStateStore();

  useEffect(() => {
    if (!subscribe || !pairId) return;
    const { addresses } = appConfig;

    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 5,
        httpInterval: 5_000,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.perps,
            msg: { pairState: { pairId: getPerpsPairId() } },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsPairState | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        setState({ pairState: response.wasmSmart, pairId: getPerpsPairId(), blockHeight });
      },
    });

    return () => unsubscribe();
  }, [appConfig.addresses, subscribe, pairId]);

  return { perpsPairStateStore };
}
