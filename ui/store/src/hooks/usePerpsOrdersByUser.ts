import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { equalLiveResourcePayload } from "../live/equality.js";
import { useLiveResource } from "../live/useLiveResource.js";

import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";

import type { PerpsOrdersByUserResponse, QueryRequest } from "@left-curve/types";
import type { Config } from "../types/store.js";
import type { LiveResourceSnapshot } from "../live/types.js";

const PERPS_ORDERS_BY_USER_INTERVAL = 5;
const PERPS_ORDERS_BY_USER_HTTP_INTERVAL = 5_000;

export type PerpsOrdersByUserSnapshot = LiveResourceSnapshot & {
  orders: PerpsOrdersByUserResponse | null;
  lastUpdatedBlockHeight: number;
};

export type UsePerpsOrdersByUserParameters = {
  accountAddress?: string;
  enabled?: boolean;
};

type PerpsOrdersByUserResourceParams = {
  accountAddress: string;
  perpsContract: string;
  subscriptions: Config["subscriptions"];
};

const initialPerpsOrdersByUserSnapshot: PerpsOrdersByUserSnapshot = {
  status: "idle",
  error: null,
  orders: null,
  lastUpdatedBlockHeight: 0,
};

const perpsOrdersByUserResource = createLiveResource<
  PerpsOrdersByUserResourceParams,
  PerpsOrdersByUserSnapshot
>({
  name: "perpsOrdersByUser",
  getKey: ({ perpsContract, accountAddress }) =>
    `perpsOrdersByUser:${perpsContract}:${accountAddress}`,
  getInitialSnapshot: () => initialPerpsOrdersByUserSnapshot,
  equal: (previous, next) => equalLiveResourcePayload(previous, next, ["orders"]),
  start: ({ accountAddress, perpsContract, subscriptions }, { emit, error }) =>
    subscriptions.subscribe("queryApp", {
      params: {
        interval: PERPS_ORDERS_BY_USER_INTERVAL,
        httpInterval: PERPS_ORDERS_BY_USER_HTTP_INTERVAL,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: perpsContract,
            msg: { ordersByUser: { user: accountAddress } },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsOrdersByUserResponse | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        emit(
          {
            status: "ready",
            error: null,
            orders: response.wasmSmart,
            lastUpdatedBlockHeight: blockHeight,
          },
          { version: blockHeight },
        );
      },
      onError: error,
    }),
});

export function usePerpsOrdersByUser<Selection>(
  selector: (snapshot: PerpsOrdersByUserSnapshot) => Selection,
  parameters: UsePerpsOrdersByUserParameters,
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection {
  const { accountAddress, enabled = true } = parameters;
  const config = useConfig();
  const { data: appConfig } = useAppConfig();

  return useLiveResource({
    resource: perpsOrdersByUserResource,
    params: {
      accountAddress: accountAddress ?? "",
      perpsContract: appConfig.addresses.perps,
      subscriptions: config.subscriptions,
    },
    enabled: enabled && !!accountAddress,
    selector,
    equalityFn,
    restartToken: config.subscriptions,
  });
}
