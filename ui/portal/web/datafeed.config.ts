import {
  CHART_RESOLUTIONS,
  convertResolutionToCandleInterval,
} from "./src/components/dex/helpers/chartResolution";
import { buildFillMarker, type FillMarker } from "./src/components/dex/helpers/fillMarkers";
import {
  getPerpsPairId,
  parseTradePairSymbols,
} from "./src/components/dex/helpers/tradePairSymbols";

import type { PerpsCandle } from "@left-curve/types";
import type { PublicClient } from "@left-curve/sdk";
import type { useConfig } from "@left-curve/store";
import type { QueryClient } from "@tanstack/react-query";
import type {
  LibrarySymbolInfo,
  ResolutionString,
  HistoryCallback,
  PeriodParams,
  OnReadyCallback,
  ResolveCallback,
  SubscribeBarsCallback,
  IBasicDataFeed,
  GetMarksCallback,
  Mark,
} from "@left-curve/tradingview";

const FILL_MARKERS_LIMIT = 500;
const FILL_MARKERS_RANGE_BUCKET_SECONDS = 24 * 60 * 60;

function perpsCandlesToTradingViewBar(candles: PerpsCandle[]) {
  return candles.reverse().map((candle) => ({
    time: candle.timeStartUnix,
    volume: +candle.volumeUsd,
    open: +candle.open,
    high: +candle.high,
    low: +candle.low,
    close: +candle.close,
  }));
}

type CreatePerpsDataFeedParameters = {
  client: PublicClient;
  queryClient: QueryClient;
  subscriptions: ReturnType<typeof useConfig>["subscriptions"];
  getAccountAddress?: () => string | undefined;
};

function perpsSymbolToPairId(symbolName: string): string {
  const parsed = parseTradePairSymbols(symbolName);
  if (!parsed) throw new Error(`Unsupported perps symbol: ${symbolName}`);
  return getPerpsPairId(parsed);
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function getFillMarkerRangeBucket(from: number, to: number) {
  return {
    from: Math.floor(from / FILL_MARKERS_RANGE_BUCKET_SECONDS) * FILL_MARKERS_RANGE_BUCKET_SECONDS,
    to: Math.ceil(to / FILL_MARKERS_RANGE_BUCKET_SECONDS) * FILL_MARKERS_RANGE_BUCKET_SECONDS,
  };
}

export function createPerpsDataFeed(parameters: CreatePerpsDataFeedParameters): IBasicDataFeed {
  const { client, queryClient, subscriptions, getAccountAddress } = parameters;

  let _unsubscribe: () => void = () => {};

  const unsubscribe: () => void = () => {
    _unsubscribe?.();
  };

  return {
    onReady: (callback: OnReadyCallback) => {
      setTimeout(
        () =>
          callback({
            supported_resolutions: [...CHART_RESOLUTIONS],
            supports_marks: true,
          }),
        0,
      );
    },

    resolveSymbol: (
      symbolName: string,
      onSymbolResolvedCallback: ResolveCallback,
      _onResolveErrorCallback: (reason: string) => void,
      _extension?: unknown,
    ) => {
      const [base] = symbolName.split("-");

      const symbolInfo: LibrarySymbolInfo = {
        name: symbolName,
        ticker: symbolName,
        description: `${base} / USD Perp`,
        session: "24x7",
        type: "crypto",
        timezone: "Etc/UTC",
        has_seconds: true,
        exchange: "Dango",
        listed_exchange: "Dango",
        format: "price",
        pricescale: 100,
        minmov: 1,
        has_intraday: true,
        supported_resolutions: [...CHART_RESOLUTIONS],
        volume_precision: 2,
        data_status: "streaming",
      };

      setTimeout(() => onSymbolResolvedCallback(symbolInfo), 0);
    },

    getBars: (
      symbolInfo: LibrarySymbolInfo,
      resolution: ResolutionString,
      periodParams: PeriodParams,
      onHistoryCallback: HistoryCallback,
      onErrorCallback: (reason: string) => void,
    ) => {
      const { to } = periodParams;
      const currentPairId = perpsSymbolToPairId(symbolInfo.name);
      const interval = convertResolutionToCandleInterval(resolution);
      const earlierThan = new Date(to * 1000);

      queryClient
        .fetchQuery({
          queryKey: ["perpsCandles", currentPairId, earlierThan, interval],
          queryFn: () =>
            client.queryPerpsCandles({
              pairId: currentPairId,
              interval,
              earlierThan: earlierThan.toJSON(),
            }),
        })
        .then(({ nodes }) => {
          const bars = perpsCandlesToTradingViewBar(nodes);

          if (bars.length > 0) {
            onHistoryCallback(bars, { noData: false });
          } else {
            onHistoryCallback([], { noData: true });
          }
        })
        .catch((error: unknown) => {
          console.error("Failed to fetch perps bars:", error);
          onErrorCallback(getErrorMessage(error));
        });
    },

    subscribeBars: (
      symbolInfo: LibrarySymbolInfo,
      resolution: ResolutionString,
      onRealtimeCallback: SubscribeBarsCallback,
      _subscriberId: string,
    ) => {
      const currentPairId = perpsSymbolToPairId(symbolInfo.name);
      const interval = convertResolutionToCandleInterval(resolution);
      unsubscribe();

      _unsubscribe = subscriptions.subscribe("perpsCandles", {
        params: { pairId: currentPairId, interval },
        listener: ({ perpsCandles }) => {
          if (perpsCandles.length > 0) {
            const [newBar] = perpsCandlesToTradingViewBar(perpsCandles);
            onRealtimeCallback(newBar);
          }
        },
      });
    },

    getMarks: (
      symbolInfo: LibrarySymbolInfo,
      from: number,
      to: number,
      onDataCallback: GetMarksCallback<Mark>,
      resolution: ResolutionString,
    ) => {
      const accountAddress = getAccountAddress?.();
      if (!accountAddress) {
        onDataCallback([]);
        return;
      }

      const currentPairId = perpsSymbolToPairId(symbolInfo.name);
      const rangeBucket = getFillMarkerRangeBucket(from, to);
      const earlierThan = new Date(rangeBucket.to * 1000);
      const laterThan = new Date(rangeBucket.from * 1000);

      queryClient
        .fetchQuery({
          queryKey: [
            "perpsTradeHistory",
            accountAddress,
            "fillMarkers",
            currentPairId,
            rangeBucket.from,
            rangeBucket.to,
            resolution,
          ],
          staleTime: 10_000,
          queryFn: () =>
            client.queryPerpsEvents({
              userAddr: accountAddress,
              pairId: currentPairId,
              eventType: "order_filled",
              sortBy: "BLOCK_HEIGHT_DESC",
              first: FILL_MARKERS_LIMIT,
              earlierThan: earlierThan.toJSON(),
              laterThan: laterThan.toJSON(),
            }),
        })
        .then(({ nodes }) => {
          const markers = nodes
            .map((event) =>
              buildFillMarker(event, {
                resolution,
              }),
            )
            .filter((marker): marker is FillMarker => marker !== null)
            .filter((marker) => marker.time >= from && marker.time <= to)
            .sort((a, b) => a.time - b.time);

          onDataCallback(markers);
        })
        .catch((error: unknown) => {
          console.error("Failed to fetch perps fill markers:", error);
          onDataCallback([]);
        });
    },

    searchSymbols: () => {},
    unsubscribeBars: () => {},
  };
}
