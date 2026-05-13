import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { useAccount } from "./useAccount.js";
import { createBlockStore } from "./createBlockStore.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { PerpsOrdersByUserResponse, QueryRequest } from "@left-curve/dango/types";

export const perpsOrdersByUserStore = createBlockStore({
  initialState: { orders: null as PerpsOrdersByUserResponse | null },
});

type UsePerpsOrdersByUserParameters = {
  subscribe?: boolean;
};

export function usePerpsOrdersByUser(parameters?: UsePerpsOrdersByUserParameters) {
  const { subscribe = true } = parameters ?? {};
  const { subscriptions } = useConfig();
  const { data: appConfig } = useAppConfig();
  const { account } = useAccount();

  const { setState } = perpsOrdersByUserStore();

  useEffect(() => {
    if (!appConfig || !subscribe || !account) return;
    const { addresses } = appConfig;

    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 5,
        httpInterval: 5_000,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.perps,
            msg: { ordersByUser: { user: account.address } },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsOrdersByUserResponse | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        setState({ orders: response.wasmSmart, blockHeight });
      },
    });

    return () => unsubscribe();
  }, [appConfig, subscribe, account]);

  return { perpsOrdersByUserStore };
}
