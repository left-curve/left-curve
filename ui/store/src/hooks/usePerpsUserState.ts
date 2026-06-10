import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";
import { useLiveResource } from "../live/useLiveResource.js";
import { usePerpsAccountResourceRevision } from "./perpsAccountResourceInvalidation.js";
import { createPerpsUserResource } from "./createPerpsUserResource.js";

import { snakeCaseJsonSerialization } from "@left-curve/encoding";
import { useMemo } from "react";

import type { PerpsUserResourceParams } from "./createPerpsUserResource.js";
import type { PerpsUserState, QueryRequest } from "@left-curve/types";
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

const initialPerpsUserStateSnapshot: PerpsUserStateSnapshot = {
  status: "idle",
  error: null,
  userState: null,
  lastUpdatedBlockHeight: 0,
};

function buildPerpsUserStateRequest({
  accountAddress,
  perpsContract,
}: Pick<PerpsUserResourceParams, "accountAddress" | "perpsContract">) {
  return snakeCaseJsonSerialization<QueryRequest>({
    wasmSmart: {
      contract: perpsContract,
      msg: { userState: { user: accountAddress } },
    },
  });
}

const perpsUserStateResource = createPerpsUserResource<PerpsUserState, PerpsUserStateSnapshot>({
  name: "perpsUserState",
  initialSnapshot: initialPerpsUserStateSnapshot,
  payloadKeys: ["userState"],
  interval: PERPS_USER_STATE_INTERVAL,
  httpInterval: PERPS_USER_STATE_HTTP_INTERVAL,
  buildRequest: buildPerpsUserStateRequest,
  mapResponse: (userState, blockHeight) => ({
    status: "ready",
    error: null,
    userState,
    lastUpdatedBlockHeight: blockHeight,
  }),
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
