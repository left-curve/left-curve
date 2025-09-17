import { useEffect } from "react";
import { useConfig } from "./useConfig.js";
import { useAppConfig } from "./useAppConfig.js";

import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";
import { Decimal, parseUnits } from "@left-curve/dango/utils";

import type {
  LiquidityDepth,
  LiquidityDepthResponse,
  PairId,
  QueryRequest,
} from "@left-curve/dango/types";
import type { AnyCoin } from "../types/coin.js";
import { create } from "zustand";

function liquidityDepthMapper(parameters: {
  records: [string, LiquidityDepth][];
  coins: { base: AnyCoin; quote: AnyCoin };
  bucketSizeCoin: "base" | "quote";
}) {
  const { coins, records, bucketSizeCoin } = parameters;
  const { base, quote } = coins;

  const isBase = bucketSizeCoin === "base";

  return records
    .sort(([priceA], [priceB]) => (Decimal(priceA).gt(priceB) ? -1 : 1))
    .reduce(
      (acc, [price, liquidityDepth]) => {
        const parsedPrice = parseUnits(price, base.decimals - quote.decimals);

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
        return acc;
      },
      Object.assign({ records: [], total: "0" }),
    );
}

type LiquidityDepthOverview = {
  total: string;
  records: { price: string; size: string; total: string }[];
};

export type LiquidityDepthStoreState = {
  bucketSizeCoin: "base" | "quote";
  setBucketSizeCoin: (symbol: "base" | "quote") => void;
  liquidityDepth?: {
    asks: LiquidityDepthOverview;
    bids: LiquidityDepthOverview;
  };
  setLiquidityDepth: (liquidityDepth: LiquidityDepthStoreState["liquidityDepth"]) => void;
};

const liquidityDepthStore = create<LiquidityDepthStoreState>((set) => ({
  bucketSizeCoin: "base",
  setBucketSizeCoin: (bucketSizeCoin) => set(() => ({ bucketSizeCoin })),
  setLiquidityDepth: (liquidityDepth) => set(() => ({ liquidityDepth })),
}));

type UseLiquidityDepthStateParameters = {
  pairId: PairId;
  subscribe?: boolean;
  bucketSize: string;
};

export function useLiquidityDepthState(parameters: UseLiquidityDepthStateParameters) {
  const { pairId, subscribe, bucketSize } = parameters;
  const { subscriptions, coins } = useConfig();
  const { data: appConfig } = useAppConfig();

  const baseCoin = coins.byDenom[pairId.baseDenom];
  const quoteCoin = coins.byDenom[pairId.quoteDenom];

  const { bucketSizeCoin, setLiquidityDepth } = liquidityDepthStore();

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
        type Event = { wasmSmart: LiquidityDepthResponse };
        const { wasmSmart: liquidityDepth } = camelCaseJsonDeserialization<Event>(event);

        const asks = liquidityDepthMapper({
          records: liquidityDepth.askDepth || [],
          coins: { base: baseCoin, quote: quoteCoin },
          bucketSizeCoin,
        });

        const bids = liquidityDepthMapper({
          records: liquidityDepth.bidDepth || [],
          coins: { base: baseCoin, quote: quoteCoin },
          bucketSizeCoin,
        });

        setLiquidityDepth({ asks, bids });
      },
    });
    return unsubscribe;
  }, [appConfig, bucketSizeCoin, bucketSize, baseCoin, quoteCoin, subscribe]);

  return { liquidityDepthStore };
}
