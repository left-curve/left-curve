import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";
import { useLiveResource } from "../live/useLiveResource.js";
import { usePerpsAccountResourceRevision } from "./perpsAccountResourceInvalidation.js";
import { createPerpsUserResource } from "./createPerpsUserResource.js";

import { snakeCaseJsonSerialization } from "@left-curve/encoding";
import { useMemo } from "react";

import type { PerpsUserResourceParams } from "./createPerpsUserResource.js";
import type {
  PerpsPositionExtended,
  PerpsUserStateExtended,
  QueryRequest,
} from "@left-curve/types";
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
}: Pick<PerpsUserResourceParams, "accountAddress" | "perpsContract">) {
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

const perpsUserStateExtendedResource = createPerpsUserResource<
  PerpsUserStateExtended,
  PerpsUserStateExtendedSnapshot
>({
  name: "perpsUserStateExtended",
  initialSnapshot: initialPerpsUserStateExtendedSnapshot,
  payloadKeys: ["equity", "availableMargin", "maintenanceMargin", "positions"],
  interval: PERPS_USER_STATE_EXTENDED_INTERVAL,
  httpInterval: PERPS_USER_STATE_EXTENDED_HTTP_INTERVAL,
  buildRequest: buildPerpsUserStateExtendedRequest,
  mapResponse: (userStateExtended, blockHeight) => ({
    status: "ready",
    error: null,
    equity: userStateExtended?.equity ?? null,
    availableMargin: userStateExtended?.availableMargin ?? null,
    maintenanceMargin: userStateExtended?.maintenanceMargin ?? null,
    positions: userStateExtended?.positions ?? {},
    lastUpdatedBlockHeight: blockHeight,
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
