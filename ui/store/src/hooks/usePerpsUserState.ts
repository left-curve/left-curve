import { useEffect } from "react";
import { create } from "zustand";
import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { useAccount } from "./useAccount.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type { PerpsUserState, QueryRequest } from "@left-curve/dango/types";

export const perpsMarginAsset = {
  name: "US Dollar",
  symbol: "USD",
  logoURI: "/images/coins/usd.svg",
  decimals: 6,
} as const;

type PerpsUserStateStoreState = {
  lastUpdatedBlockHeight: number;
  userState: PerpsUserState | null;
  setState: (params: { userState: PerpsUserState | null; blockHeight: number }) => void;
};

export const perpsUserStateStore = create<PerpsUserStateStoreState>((set, get) => ({
  lastUpdatedBlockHeight: 0,
  userState: null,
  setState: ({ userState, blockHeight }) => {
    const { lastUpdatedBlockHeight } = get();
    if (blockHeight <= lastUpdatedBlockHeight) return;
    set({ userState, lastUpdatedBlockHeight: blockHeight });
  },
}));

type UsePerpsUserStateParameters = {
  subscribe?: boolean;
};

export function usePerpsUserState(parameters?: UsePerpsUserStateParameters) {
  const { subscribe = true } = parameters ?? {};
  const { subscriptions } = useConfig();
  const { data: appConfig } = useAppConfig();
  const { account } = useAccount();

  const { setState } = perpsUserStateStore();

  useEffect(() => {
    if (!appConfig || !subscribe || !account) return;
    const { addresses } = appConfig;

    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 5,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.perps,
            msg: { userState: { user: account.address } },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsUserState | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        setState({ userState: response.wasmSmart, blockHeight });
      },
    });

    return () => unsubscribe();
  }, [appConfig, subscribe, account]);

  return { perpsUserStateStore };
}
