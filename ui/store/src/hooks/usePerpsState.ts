import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { equalLiveResourcePayload } from "../live/equality.js";
import { useLiveResource } from "../live/useLiveResource.js";

import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";

import type { PerpsState, QueryRequest } from "@left-curve/types";
import type { Config } from "../types/store.js";
import type { LiveResourceSnapshot } from "../live/types.js";

const PERPS_STATE_INTERVAL = 5;
const PERPS_STATE_HTTP_INTERVAL = 5_000;

export type PerpsStateSnapshot = LiveResourceSnapshot & {
  state: PerpsState | null;
  lastUpdatedBlockHeight: number;
};

export type UsePerpsStateParameters = {
  enabled?: boolean;
};

type PerpsStateResourceParams = {
  chainId: Config["chain"]["id"];
  perpsContract: string;
  subscriptions: Config["subscriptions"];
};

const initialPerpsStateSnapshot: PerpsStateSnapshot = {
  status: "idle",
  error: null,
  state: null,
  lastUpdatedBlockHeight: 0,
};

const perpsStateResource = createLiveResource<PerpsStateResourceParams, PerpsStateSnapshot>({
  name: "perpsState",
  getKey: ({ chainId, perpsContract }) => `perpsState:${chainId}:${perpsContract}`,
  getInitialSnapshot: () => initialPerpsStateSnapshot,
  equal: (previous, next) => equalLiveResourcePayload(previous, next, ["state"]),
  start: ({ perpsContract, subscriptions }, { emit, error }) =>
    subscriptions.subscribe("queryApp", {
      params: {
        interval: PERPS_STATE_INTERVAL,
        httpInterval: PERPS_STATE_HTTP_INTERVAL,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: perpsContract,
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
        emit(
          {
            status: "ready",
            error: null,
            state: response.wasmSmart,
            lastUpdatedBlockHeight: blockHeight,
          },
          { version: blockHeight },
        );
      },
      onError: error,
    }),
});

export function usePerpsState<Selection>(
  selector: (snapshot: PerpsStateSnapshot) => Selection,
  parameters: UsePerpsStateParameters = {},
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection {
  const { enabled = true } = parameters;
  const config = useConfig();
  const { data: appConfig } = useAppConfig();

  return useLiveResource({
    resource: perpsStateResource,
    params: {
      chainId: config.chain.id,
      perpsContract: appConfig.addresses.perps,
      subscriptions: config.subscriptions,
    },
    enabled,
    selector,
    equalityFn,
    restartToken: config.subscriptions,
  });
}
