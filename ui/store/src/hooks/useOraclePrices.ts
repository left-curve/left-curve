import { useAppConfig } from "./useAppConfig.js";
import { useConfig } from "./useConfig.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { useLiveResource } from "../live/useLiveResource.js";

import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";

import type { Denom, Price, QueryRequest } from "@left-curve/types";
import type { Config } from "../types/store.js";
import type { LiveResourceSnapshot } from "../live/types.js";

const ORACLE_PRICES_INTERVAL = 1;
const ORACLE_PRICES_HTTP_INTERVAL = 2_000;

export type OraclePricesSnapshot = LiveResourceSnapshot & {
  prices: Record<Denom, Price>;
  lastUpdatedBlockHeight: number;
};

export type UseOraclePricesParameters = {
  enabled?: boolean;
};

type OraclePricesResourceParams = {
  chainId: Config["chain"]["id"];
  oracleContract: string;
  subscriptions: Config["subscriptions"];
};

const initialOraclePricesSnapshot: OraclePricesSnapshot = {
  status: "idle",
  error: null,
  prices: {},
  lastUpdatedBlockHeight: 0,
};

function equalPrice(previous: Price, next: Price) {
  return (
    previous.humanizedPrice === next.humanizedPrice &&
    previous.timestamp === next.timestamp &&
    previous.marketSession === next.marketSession
  );
}

function equalOraclePricesSnapshot(previous: OraclePricesSnapshot, next: OraclePricesSnapshot) {
  if (
    previous.status !== next.status ||
    previous.error !== next.error ||
    previous.lastUpdatedBlockHeight !== next.lastUpdatedBlockHeight
  ) {
    return false;
  }

  const previousEntries = Object.entries(previous.prices);
  const nextEntries = Object.entries(next.prices);
  if (previousEntries.length !== nextEntries.length) return false;

  for (const [denom, price] of previousEntries) {
    const nextPrice = next.prices[denom];
    if (!nextPrice || !equalPrice(price, nextPrice)) return false;
  }

  return true;
}

const oraclePricesResource = createLiveResource<OraclePricesResourceParams, OraclePricesSnapshot>({
  name: "oraclePrices",
  getKey: ({ chainId, oracleContract }) => `oraclePrices:${chainId}:${oracleContract}`,
  getInitialSnapshot: () => initialOraclePricesSnapshot,
  equal: equalOraclePricesSnapshot,
  start: ({ oracleContract, subscriptions }, { emit, error }) =>
    subscriptions.subscribe("queryApp", {
      params: {
        interval: ORACLE_PRICES_INTERVAL,
        httpInterval: ORACLE_PRICES_HTTP_INTERVAL,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: oracleContract,
            msg: {
              prices: {},
            },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: Record<Denom, Price> };
          blockHeight: number;
        };

        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        const { wasmSmart: prices } = response;

        emit(
          {
            status: "ready",
            error: null,
            prices,
            lastUpdatedBlockHeight: blockHeight,
          },
          { version: blockHeight },
        );
      },
      onError: error,
    }),
});

export function useOraclePrices<Selection>(
  selector: (snapshot: OraclePricesSnapshot) => Selection,
  parameters: UseOraclePricesParameters = {},
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection {
  const { enabled = true } = parameters;
  const config = useConfig();
  const { data: appConfig } = useAppConfig();

  return useLiveResource({
    resource: oraclePricesResource,
    params: {
      chainId: config.chain.id,
      oracleContract: appConfig.addresses.oracle,
      subscriptions: config.subscriptions,
    },
    enabled,
    selector,
    equalityFn,
    restartToken: config.subscriptions,
  });
}
