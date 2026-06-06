import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { equalLiveResourcePayload } from "../live/equality.js";
import { useLiveResource } from "../live/useLiveResource.js";

import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";

import type { PerpsUserState, QueryRequest } from "@left-curve/types";
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
  subscriptions: Config["subscriptions"];
};

const initialPerpsUserStateSnapshot: PerpsUserStateSnapshot = {
  status: "idle",
  error: null,
  userState: null,
  lastUpdatedBlockHeight: 0,
};

const perpsUserStateResource = createLiveResource<
  PerpsUserStateResourceParams,
  PerpsUserStateSnapshot
>({
  name: "perpsUserState",
  getKey: ({ chainId, perpsContract, accountAddress }) =>
    `perpsUserState:${chainId}:${perpsContract}:${accountAddress}`,
  getInitialSnapshot: () => initialPerpsUserStateSnapshot,
  equal: (previous, next) => equalLiveResourcePayload(previous, next, ["userState"]),
  start: ({ accountAddress, perpsContract, subscriptions }, { emit, error }) =>
    subscriptions.subscribe("queryApp", {
      params: {
        interval: PERPS_USER_STATE_INTERVAL,
        httpInterval: PERPS_USER_STATE_HTTP_INTERVAL,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: perpsContract,
            msg: { userState: { user: accountAddress } },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsUserState | null };
          blockHeight: number;
        };
        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        emit(
          {
            status: "ready",
            error: null,
            userState: response.wasmSmart,
            lastUpdatedBlockHeight: blockHeight,
          },
          { version: blockHeight },
        );
      },
      onError: error,
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

  return useLiveResource({
    resource: perpsUserStateResource,
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
