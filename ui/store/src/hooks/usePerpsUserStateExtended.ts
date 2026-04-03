import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { useAccount } from "./useAccount.js";
import { createBlockStore } from "./createBlockStore.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { PerpsUserStateExtended, QueryRequest } from "@left-curve/dango/types";

export const perpsUserStateExtendedStore = createBlockStore({
  initialState: { availableMargin: null as string | null },
});

type UsePerpsUserStateExtendedParameters = {
  subscribe?: boolean;
};

export function usePerpsUserStateExtended(
  parameters?: UsePerpsUserStateExtendedParameters,
) {
  const { subscribe = true } = parameters ?? {};
  const { subscriptions } = useConfig();
  const { data: appConfig } = useAppConfig();
  const { account } = useAccount();

  const { setState } = perpsUserStateExtendedStore();

  useEffect(() => {
    if (!subscribe || !account) return;
    const { addresses } = appConfig;

    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 5,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.perps,
            msg: {
              userStateExtended: {
                user: account.address,
                includeEquity: false,
                includeAvailableMargin: true,
              },
            },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsUserStateExtended | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        setState({
          availableMargin: response.wasmSmart?.availableMargin ?? null,
          blockHeight,
        });
      },
    });

    return () => unsubscribe();
  }, [appConfig.addresses, subscribe, account]);

  return { perpsUserStateExtendedStore };
}
