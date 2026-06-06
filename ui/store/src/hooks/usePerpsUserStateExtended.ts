import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { useLiveResource } from "../live/useLiveResource.js";

import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";

import type {
  PerpsPositionExtended,
  PerpsUserStateExtended,
  QueryRequest,
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

const perpsUserStateExtendedResource = createLiveResource<
  PerpsUserStateExtendedResourceParams,
  PerpsUserStateExtendedSnapshot
>({
  name: "perpsUserStateExtended",
  getKey: ({ chainId, perpsContract, accountAddress }) =>
    `perpsUserStateExtended:${chainId}:${perpsContract}:${accountAddress}`,
  getInitialSnapshot: () => initialPerpsUserStateExtendedSnapshot,
  start: ({ accountAddress, perpsContract, subscriptions }, { emit, error }) =>
    subscriptions.subscribe("queryApp", {
      params: {
        interval: PERPS_USER_STATE_EXTENDED_INTERVAL,
        httpInterval: PERPS_USER_STATE_EXTENDED_HTTP_INTERVAL,
        request: snakeCaseJsonSerialization<QueryRequest>({
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
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsUserStateExtended | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        emit(
          {
            status: "ready",
            error: null,
            equity: response.wasmSmart?.equity ?? null,
            availableMargin: response.wasmSmart?.availableMargin ?? null,
            maintenanceMargin: response.wasmSmart?.maintenanceMargin ?? null,
            positions: response.wasmSmart?.positions ?? {},
            lastUpdatedBlockHeight: blockHeight,
          },
          { version: blockHeight },
        );
      },
      onError: error,
    }),
});

export function usePerpsUserStateExtended<Selection>(
  selector: (snapshot: PerpsUserStateExtendedSnapshot) => Selection,
  parameters: UsePerpsUserStateExtendedParameters,
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection {
  const { accountAddress, enabled = true } = parameters;
  const config = useConfig();
  const { data: appConfig } = useAppConfig();

  return useLiveResource({
    resource: perpsUserStateExtendedResource,
    params: {
      chainId: config.chain.id,
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
