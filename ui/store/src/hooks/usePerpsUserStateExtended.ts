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
import type {
  PerpsPositionExtended,
  PerpsUserStateExtended,
  QueryRequest,
  QueryResponse,
} from "@left-curve/types";
import type { Config } from "../types/store.js";
import type { LiveResourceSnapshot } from "../live/types.js";

const PERPS_USER_STATE_EXTENDED_INTERVAL = 5;
const PERPS_USER_STATE_EXTENDED_HTTP_INTERVAL = 10_000;

export type PerpsUserStateExtendedSnapshot = LiveResourceSnapshot & {
  equity: string | null;
  availableMargin: string | null;
  maintenanceMargin: string | null;
  positions: Record<string, PerpsPositionExtended>;
  lastUpdatedBlockHeight: number;
};

export type UsePerpsUserStateExtendedParameters = {
  accountAddress?: string;
  enabled?: boolean;
};

type PerpsUserStateExtendedResourceParams = {
  chainId: Config["chain"]["id"];
  accountAddress: string;
  perpsContract: string;
  publicClient: PublicClient;
  subscriptions: Config["subscriptions"];
};

const initialPerpsUserStateExtendedSnapshot: PerpsUserStateExtendedSnapshot = {
  status: "idle",
  error: null,
  equity: null,
  availableMargin: null,
  maintenanceMargin: null,
  positions: {},
  lastUpdatedBlockHeight: 0,
};

function buildPerpsUserStateExtendedRequest({
  accountAddress,
  perpsContract,
}: Pick<PerpsUserStateExtendedResourceParams, "accountAddress" | "perpsContract">) {
  return snakeCaseJsonSerialization<QueryRequest>({
    wasmSmart: {
      contract: perpsContract,
      msg: {
        userStateExtended: {
          user: accountAddress,
          includeEquity: true,
          includeAvailableMargin: true,
          includeMaintenanceMargin: true,
          includeUnrealizedPnl: true,
          includeUnrealizedFunding: true,
          includeLiquidationPrice: true,
          includeAll: true,
        },
      },
    },
  });
}

function getPerpsUserStateExtendedResponse(response: QueryResponse) {
  if (!("wasmSmart" in response)) {
    throw new Error(`expecting wasm smart response, got ${JSON.stringify(response)}`);
  }

  return response.wasmSmart as PerpsUserStateExtended | null;
}

const perpsUserStateExtendedResource = createLiveResource<
  PerpsUserStateExtendedResourceParams,
  PerpsUserStateExtendedSnapshot
>({
  name: "perpsUserStateExtended",
  getKey: ({ chainId, perpsContract, accountAddress }) =>
    `perpsUserStateExtended:${chainId}:${perpsContract}:${accountAddress}`,
  getInitialSnapshot: () => initialPerpsUserStateExtendedSnapshot,
  equal: (previous, next) =>
    equalLiveResourcePayload(previous, next, [
      "equity",
      "availableMargin",
      "maintenanceMargin",
      "positions",
    ]),
  start: ({ accountAddress, perpsContract, publicClient, subscriptions }, { emit, error }) => {
    const request = buildPerpsUserStateExtendedRequest({ accountAddress, perpsContract });
    let stopped = false;
    let receivedSubscriptionEvent = false;

    const emitUserStateExtended = (
      userStateExtended: PerpsUserStateExtended | null,
      blockHeight: number,
      options?: { version: number },
    ) =>
      emit(
        {
          status: "ready",
          error: null,
          equity: userStateExtended?.equity ?? null,
          availableMargin: userStateExtended?.availableMargin ?? null,
          maintenanceMargin: userStateExtended?.maintenanceMargin ?? null,
          positions: userStateExtended?.positions ?? {},
          lastUpdatedBlockHeight: blockHeight,
        },
        options,
      );

    void publicClient
      .queryApp({ query: request })
      .then((response) => {
        if (stopped || receivedSubscriptionEvent) return;
        emitUserStateExtended(getPerpsUserStateExtendedResponse(response), 0);
      })
      .catch((caught) => {
        if (!stopped) error(caught);
      });

    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: PERPS_USER_STATE_EXTENDED_INTERVAL,
        httpInterval: PERPS_USER_STATE_EXTENDED_HTTP_INTERVAL,
        request,
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsUserStateExtended | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        receivedSubscriptionEvent = true;
        emitUserStateExtended(response.wasmSmart, blockHeight, { version: blockHeight });
      },
      onError: error,
    });

    return () => {
      stopped = true;
      unsubscribe();
    };
  },
});

export function usePerpsUserStateExtended<Selection>(
  selector: (snapshot: PerpsUserStateExtendedSnapshot) => Selection,
  parameters: UsePerpsUserStateExtendedParameters,
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
    resource: perpsUserStateExtendedResource,
    params: resourceParams,
    enabled: enabled && !!accountAddress,
    selector,
    equalityFn,
    restartToken,
  });
}
