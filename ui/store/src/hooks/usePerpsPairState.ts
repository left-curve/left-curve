import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { useLiveResource } from "../live/useLiveResource.js";

import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";

import type { PerpsPairState, QueryRequest } from "@left-curve/types";
import type { Config } from "../types/store.js";
import type { LiveResourceSnapshot } from "../live/types.js";

const PERPS_PAIR_STATE_INTERVAL = 5;
const PERPS_PAIR_STATE_HTTP_INTERVAL = 5_000;

export type PerpsPairStateSnapshot = LiveResourceSnapshot & {
  pairState: PerpsPairState | null;
  pairId: string | null;
  lastUpdatedBlockHeight: number;
};

export type UsePerpsPairStateParameters = {
  perpsPairId?: string;
  enabled?: boolean;
};

type PerpsPairStateResourceParams = {
  chainId: Config["chain"]["id"];
  perpsPairId: string;
  perpsContract: string;
  subscriptions: Config["subscriptions"];
};

const initialPerpsPairStateSnapshot: PerpsPairStateSnapshot = {
  status: "idle",
  error: null,
  pairState: null,
  pairId: null,
  lastUpdatedBlockHeight: 0,
};

const perpsPairStateResource = createLiveResource<
  PerpsPairStateResourceParams,
  PerpsPairStateSnapshot
>({
  name: "perpsPairState",
  getKey: ({ chainId, perpsContract, perpsPairId }) =>
    `perpsPairState:${chainId}:${perpsContract}:${perpsPairId}`,
  getInitialSnapshot: () => initialPerpsPairStateSnapshot,
  start: ({ perpsPairId, perpsContract, subscriptions }, { emit, error }) =>
    subscriptions.subscribe("queryApp", {
      params: {
        interval: PERPS_PAIR_STATE_INTERVAL,
        httpInterval: PERPS_PAIR_STATE_HTTP_INTERVAL,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: perpsContract,
            msg: { pairState: { pairId: perpsPairId } },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsPairState | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        emit(
          {
            status: "ready",
            error: null,
            pairState: response.wasmSmart,
            pairId: perpsPairId,
            lastUpdatedBlockHeight: blockHeight,
          },
          { version: blockHeight },
        );
      },
      onError: error,
    }),
});

export function usePerpsPairState<Selection>(
  selector: (snapshot: PerpsPairStateSnapshot) => Selection,
  parameters: UsePerpsPairStateParameters,
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection {
  const { perpsPairId, enabled = true } = parameters;
  const config = useConfig();
  const { data: appConfig } = useAppConfig();

  return useLiveResource({
    resource: perpsPairStateResource,
    params: {
      chainId: config.chain.id,
      perpsPairId: perpsPairId ?? "",
      perpsContract: appConfig.addresses.perps,
      subscriptions: config.subscriptions,
    },
    enabled: enabled && !!perpsPairId,
    selector,
    equalityFn,
    restartToken: config.subscriptions,
  });
}
