import { useEffect } from "react";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { useAccount } from "./useAccount.js";
import { createBlockStore } from "./createBlockStore.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type {
  PerpsPositionExtended,
  PerpsUserStateExtended,
  QueryRequest,
} from "@left-curve/dango/types";

export const perpsUserStateExtendedStore = createBlockStore({
  initialState: {
    equity: null as string | null,
    availableMargin: null as string | null,
    maintenanceMargin: null as string | null,
    positions: {} as Record<string, PerpsPositionExtended>,
  },
});

type UsePerpsUserStateExtendedParameters = {
  subscribe?: boolean;
  includeEquity?: boolean;
  includeAvailableMargin?: boolean;
  includeMaintenanceMargin?: boolean;
  includeUnrealizedPnl?: boolean;
  includeUnrealizedFunding?: boolean;
  includeLiquidationPrice?: boolean;
  includeAll?: boolean;
};

export function usePerpsUserStateExtended(parameters?: UsePerpsUserStateExtendedParameters) {
  const {
    subscribe = true,
    includeEquity = true,
    includeAvailableMargin = true,
    includeMaintenanceMargin = true,
    includeUnrealizedPnl = true,
    includeUnrealizedFunding = true,
    includeLiquidationPrice = true,
    includeAll = true,
  } = parameters ?? {};
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
        httpInterval: 10_000,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.perps,
            msg: {
              userStateExtended: {
                user: account.address,
                includeEquity,
                includeAvailableMargin,
                includeMaintenanceMargin,
                includeUnrealizedPnl,
                includeUnrealizedFunding,
                includeLiquidationPrice,
                includeAll,
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
          equity: response.wasmSmart?.equity ?? null,
          availableMargin: response.wasmSmart?.availableMargin ?? null,
          maintenanceMargin: response.wasmSmart?.maintenanceMargin ?? null,
          positions: response.wasmSmart?.positions ?? {},
          blockHeight,
        });
      },
    });

    return () => unsubscribe();
  }, [
    appConfig.addresses,
    subscribe,
    account,
    includeEquity,
    includeAvailableMargin,
    includeMaintenanceMargin,
    includeUnrealizedPnl,
    includeUnrealizedFunding,
    includeLiquidationPrice,
    includeAll,
  ]);

  return { perpsUserStateExtendedStore };
}
