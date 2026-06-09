import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { equalLiveResourcePayload } from "../live/equality.js";
import { useLiveResource } from "../live/useLiveResource.js";

import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";

import type { PerpsLiquidityDepthResponse, QueryRequest } from "@left-curve/types";
import type { Config } from "../types/store.js";
import type { LiveResourceSnapshot } from "../live/types.js";

const PERPS_LIQUIDITY_DEPTH_INTERVAL = 1;
const PERPS_LIQUIDITY_DEPTH_HTTP_INTERVAL = 2_000;

export type PerpsLiquidityDepthSnapshot = LiveResourceSnapshot & {
  liquidityDepth: PerpsLiquidityDepthResponse | null;
  lastUpdatedBlockHeight: number;
};

export type UsePerpsLiquidityDepthParameters = {
  perpsPairId?: string;
  bucketSize: string;
  limit?: number;
  enabled?: boolean;
};

type PerpsLiquidityDepthResourceParams = {
  chainId: Config["chain"]["id"];
  perpsPairId: string;
  bucketSize: string;
  limit: number;
  perpsContract: string;
  subscriptions: Config["subscriptions"];
};

const initialPerpsLiquidityDepthSnapshot: PerpsLiquidityDepthSnapshot = {
  status: "idle",
  error: null,
  liquidityDepth: null,
  lastUpdatedBlockHeight: 0,
};

const perpsLiquidityDepthResource = createLiveResource<
  PerpsLiquidityDepthResourceParams,
  PerpsLiquidityDepthSnapshot
>({
  name: "perpsLiquidityDepth",
  getKey: ({ chainId, perpsContract, perpsPairId, bucketSize, limit }) =>
    `perpsLiquidityDepth:${chainId}:${perpsContract}:${perpsPairId}:${bucketSize}:${limit}`,
  getInitialSnapshot: () => initialPerpsLiquidityDepthSnapshot,
  equal: (previous, next) => equalLiveResourcePayload(previous, next, ["liquidityDepth"]),
  start: ({ perpsPairId, bucketSize, limit, perpsContract, subscriptions }, { emit, error }) =>
    subscriptions.subscribe("queryApp", {
      params: {
        interval: PERPS_LIQUIDITY_DEPTH_INTERVAL,
        httpInterval: PERPS_LIQUIDITY_DEPTH_HTTP_INTERVAL,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: perpsContract,
            msg: {
              liquidityDepth: {
                pairId: perpsPairId,
                bucketSize,
                limit,
              },
            },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: PerpsLiquidityDepthResponse };
          blockHeight: number;
        };

        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        const { wasmSmart: liquidityDepth } = response;

        emit(
          {
            status: "ready",
            error: null,
            liquidityDepth,
            lastUpdatedBlockHeight: blockHeight,
          },
          { version: blockHeight },
        );
      },
      onError: error,
    }),
});

export function usePerpsLiquidityDepth<Selection>(
  selector: (snapshot: PerpsLiquidityDepthSnapshot) => Selection,
  parameters: UsePerpsLiquidityDepthParameters,
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection {
  const { perpsPairId, bucketSize, limit = 20, enabled = true } = parameters;
  const config = useConfig();
  const { data: appConfig } = useAppConfig();

  return useLiveResource({
    resource: perpsLiquidityDepthResource,
    params: {
      chainId: config.chain.id,
      perpsPairId: perpsPairId ?? "",
      bucketSize,
      limit,
      perpsContract: appConfig.addresses.perps,
      subscriptions: config.subscriptions,
    },
    enabled: enabled && !!perpsPairId && !!bucketSize,
    selector,
    equalityFn,
    restartToken: config.subscriptions,
  });
}
