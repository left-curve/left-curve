import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { equalLiveResourcePayload } from "../live/equality.js";
import { useLiveResource } from "../live/useLiveResource.js";
import { usePerpsAccountResourceRevision } from "./perpsAccountResourceInvalidation.js";

import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";
import { useMemo } from "react";

import type { PublicClient } from "@left-curve/sdk";
import type { PerpsUserState, QueryRequest, QueryResponse } from "@left-curve/types";
import type { Config } from "../types/store.js";
import type { LiveResourceSnapshot } from "../live/types.js";

export const perpsMarginAsset = {
  name: "US Dollar",
  symbol: "USD",
  logoURI: "/images/coins/usd.svg",
  decimals: 6,
} as const;

export type PerpsAssetClass = "crypto" | "commodity";

const PERPS_COMMODITY_BASE_SYMBOLS: ReadonlySet<string> = new Set(["XAU", "XAG"]);

export function getPerpsAssetClass(baseSymbol: string): PerpsAssetClass {
  return PERPS_COMMODITY_BASE_SYMBOLS.has(baseSymbol.toUpperCase()) ? "commodity" : "crypto";
}

const PERPS_USER_STATE_INTERVAL = 5;
const PERPS_USER_STATE_HTTP_INTERVAL = 10_000;

export type PerpsUserStateSnapshot = LiveResourceSnapshot & {
  userState: PerpsUserState | null;
  lastUpdatedBlockHeight: number;
};

export type UsePerpsUserStateParameters = {
  accountAddress?: string;
  enabled?: boolean;
};

type PerpsUserStateResourceParams = {
  chainId: Config["chain"]["id"];
  accountAddress: string;
  perpsContract: string;
  publicClient: PublicClient;
  subscriptions: Config["subscriptions"];
};

const initialPerpsUserStateSnapshot: PerpsUserStateSnapshot = {
  status: "idle",
  error: null,
  userState: null,
  lastUpdatedBlockHeight: 0,
};

function buildPerpsUserStateRequest({
  accountAddress,
  perpsContract,
}: Pick<PerpsUserStateResourceParams, "accountAddress" | "perpsContract">) {
  return snakeCaseJsonSerialization<QueryRequest>({
    wasmSmart: {
      contract: perpsContract,
      msg: { userState: { user: accountAddress } },
    },
  });
}

function getPerpsUserStateResponse(response: QueryResponse) {
  if (!("wasmSmart" in response)) {
    throw new Error(`expecting wasm smart response, got ${JSON.stringify(response)}`);
  }

  return response.wasmSmart as PerpsUserState | null;
}

const perpsUserStateResource = createLiveResource<
  PerpsUserStateResourceParams,
  PerpsUserStateSnapshot
>({
  name: "perpsUserState",
  getKey: ({ chainId, perpsContract, accountAddress }) =>
    `perpsUserState:${chainId}:${perpsContract}:${accountAddress}`,
  getInitialSnapshot: () => initialPerpsUserStateSnapshot,
  equal: (previous, next) => equalLiveResourcePayload(previous, next, ["userState"]),
  start: ({ accountAddress, perpsContract, publicClient, subscriptions }, { emit, error }) => {
    const request = buildPerpsUserStateRequest({ accountAddress, perpsContract });
    let stopped = false;
    let receivedSubscriptionEvent = false;

    const emitUserState = (
      userState: PerpsUserState | null,
      blockHeight: number,
      options?: { version: number },
    ) =>
      emit(
        {
          status: "ready",
          error: null,
          userState,
          lastUpdatedBlockHeight: blockHeight,
        },
        options,
      );

    void publicClient
      .queryApp({ query: request })
      .then((response) => {
        if (stopped || receivedSubscriptionEvent) return;
        emitUserState(getPerpsUserStateResponse(response), 0);
      })
      .catch((caught) => {
        if (!stopped) error(caught);
      });

    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: PERPS_USER_STATE_INTERVAL,
        httpInterval: PERPS_USER_STATE_HTTP_INTERVAL,
        request,
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsUserState | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        receivedSubscriptionEvent = true;
        emitUserState(response.wasmSmart, blockHeight, { version: blockHeight });
      },
      onError: error,
    });

    return () => {
      stopped = true;
      unsubscribe();
    };
  },
});

export function usePerpsUserState<Selection>(
  selector: (snapshot: PerpsUserStateSnapshot) => Selection,
  parameters: UsePerpsUserStateParameters,
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection {
  const { accountAddress, enabled = true } = parameters;
  const config = useConfig();
  const { data: appConfig } = useAppConfig();
  const publicClient = usePublicClient();
  const resourceParams = {
    chainId: config.chain.id,
    accountAddress: accountAddress ?? "",
    perpsContract: appConfig.addresses.perps,
    publicClient,
    subscriptions: config.subscriptions,
  };
  const resourceRevision = usePerpsAccountResourceRevision(resourceParams);
  const restartToken = useMemo(
    () => ({ subscriptions: config.subscriptions, resourceRevision }),
    [config.subscriptions, resourceRevision],
  );

  return useLiveResource({
    resource: perpsUserStateResource,
    params: resourceParams,
    enabled: enabled && !!accountAddress,
    selector,
    equalityFn,
    restartToken,
  });
}
