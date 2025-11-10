import { useEffect } from "react";
import { useConfig } from "./useConfig.js";
import { useAppConfig } from "./useAppConfig.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";
import { Decimal } from "@left-curve/dango/utils";

import {
  Direction,
  type Directions,
  type LiquidityDepth,
  type LiquidityDepthResponse,
  type PairId,
  type QueryRequest,
} from "@left-curve/dango/types";
import type { AnyCoin } from "../types/coin.js";
import { create } from "zustand";

function liquidityDepthMapper(parameters: {
  records: [string, LiquidityDepth][];
  direction: Directions;
  coins: { base: AnyCoin; quote: AnyCoin };
  bucketSizeCoin: "base" | "quote";
  bucketRecords: number;
}) {
  const { coins, records, direction, bucketSizeCoin, bucketRecords } = parameters;
  const { base, quote } = coins;

  const isBase = bucketSizeCoin === "base";

  const sortedRecords = records
    .sort(([priceA], [priceB]) => {
      if (direction === Direction.Buy) return Decimal(priceA).gt(priceB) ? -1 : 1;
      return Decimal(priceA).gt(priceB) ? 1 : -1;
    })
    .slice(0, bucketRecords);
  return sortedRecords.reduce(
    (acc, [price, liquidityDepth]) => {
      const parsedPrice = Decimal(price).mul(Decimal(10).pow(base.decimals - quote.decimals));

      const size = Decimal(isBase ? liquidityDepth.depthBase : liquidityDepth.depthQuote).div(
        Decimal(10).pow(isBase ? base.decimals : quote.decimals),
      );

      const total = Decimal(acc.total).plus(size).toFixed();

      acc.records.push({
        price: parsedPrice,
        size: size.toFixed(),
        total: total,
      });

      acc.total = total;
      acc.highestSize = size.lt(acc.highestSize) ? acc.highestSize : size.toFixed();
      return acc;
    },
    Object.assign({ records: [], total: "0", highestSize: "0" }),
  );
}

type LiquidityDepthOverview = {
  total: string;
  highestSize: string;
  records: { price: string; size: string; total: string }[];
};

export type LiquidityDepthStoreState = {
  lastUpdatedBlockHeight: number;
  bucketSizeCoin: "base" | "quote";
  setBucketSizeCoin: (symbol: "base" | "quote") => void;
  liquidityDepth?: {
    asks: LiquidityDepthOverview;
    bids: LiquidityDepthOverview;
  };
  setLiquidityDepth: (
    liquidityDepth: LiquidityDepthStoreState["liquidityDepth"] & { blockHeight: number },
  ) => void;
};

export const liquidityDepthStore = create<LiquidityDepthStoreState>((set, get) => ({
  lastUpdatedBlockHeight: 0,
  bucketSizeCoin: "base",
  setBucketSizeCoin: (bucketSizeCoin) => set(() => ({ bucketSizeCoin })),
  setLiquidityDepth: ({ blockHeight, ...liquidityDepth }) => {
    const { lastUpdatedBlockHeight } = get();
    if (blockHeight > lastUpdatedBlockHeight) {
      set({ liquidityDepth, lastUpdatedBlockHeight: blockHeight });
    }
  },
}));

type UseLiquidityDepthStateParameters = {
  pairId: PairId;
  subscribe?: boolean;
  bucketRecords: number;
  bucketSize: string;
};

export function useLiquidityDepthState(parameters: UseLiquidityDepthStateParameters) {
  const { pairId, subscribe, bucketSize, bucketRecords } = parameters;
  const { subscriptions, coins } = useConfig();
  const { data: appConfig } = useAppConfig();

  const baseCoin = coins.byDenom[pairId.baseDenom];
  const quoteCoin = coins.byDenom[pairId.quoteDenom];

  useEffect(() => {
    if (!appConfig || !subscribe) return;
    const { addresses } = appConfig;
    const unsubscribe = subscriptions.subscribe("queryApp", {
      params: {
        interval: 1,
        request: snakeCaseJsonSerialization<QueryRequest>({
          wasmSmart: {
            contract: addresses.dex,
            msg: {
              liquidityDepth: {
                baseDenom: baseCoin.denom,
                quoteDenom: quoteCoin.denom,
                bucketSize,
              },
            },
          },
        }),
      },
      listener: (event) => {
        type Event = {
          response: { wasmSmart: LiquidityDepthResponse };
          blockHeight: number;
        };

        const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
        const { wasmSmart: liquidityDepth } = response;

        const { bucketSizeCoin, setLiquidityDepth } = liquidityDepthStore.getState();

        const asks = liquidityDepthMapper({
          records: liquidityDepth.askDepth || [],
          direction: Direction.Sell,
          coins: { base: baseCoin, quote: quoteCoin },
          bucketSizeCoin,
          bucketRecords,
        });

        const bids = liquidityDepthMapper({
          records: liquidityDepth.bidDepth || [],
          direction: Direction.Buy,
          coins: { base: baseCoin, quote: quoteCoin },
          bucketSizeCoin,
          bucketRecords,
        });

        setLiquidityDepth({ asks, bids, blockHeight });
      },
    });
    return unsubscribe;
  }, [appConfig, bucketRecords, bucketSize, baseCoin, quoteCoin, subscribe]);

  return { liquidityDepthStore };
}
