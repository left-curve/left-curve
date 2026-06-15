import {
  CHART_RESOLUTIONS,
  convertResolutionToCandleInterval,
} from "~/components/dex/helpers/chartResolution";
import { fetchFillMarkers } from "~/components/dex/helpers/fillMarkers";

import { MarketPair } from "@left-curve/foundation/market-pair";
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
  getAccountAddress: () => string | undefined;
};

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function pairFromSymbolInfo(symbolInfo: LibrarySymbolInfo): MarketPair | null {
  return MarketPair.tryFromTicker(symbolInfo.ticker || symbolInfo.name);
}

function createSymbolInfo(pair: MarketPair): LibrarySymbolInfo {
  return {
    name: pair.ticker,
    ticker: pair.ticker,
    description: `${pair.ticker} Perp`,
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
      onResolveErrorCallback: (reason: string) => void,
      _extension?: unknown,
    ) => {
      const pair = MarketPair.tryFromTicker(symbolName);
      if (!pair) {
        onResolveErrorCallback(`Unknown symbol: ${symbolName}`);
        return;
      }

      setTimeout(() => onSymbolResolvedCallback(createSymbolInfo(pair)), 0);
    },

    getBars: (
      symbolInfo: LibrarySymbolInfo,
      resolution: ResolutionString,
      periodParams: PeriodParams,
      onHistoryCallback: HistoryCallback,
      onErrorCallback: (reason: string) => void,
    ) => {
      const pair = pairFromSymbolInfo(symbolInfo);
      if (!pair) {
        onErrorCallback(`Unknown symbol: ${symbolInfo.ticker || symbolInfo.name}`);
        return;
      }

      const { to } = periodParams;
      const interval = convertResolutionToCandleInterval(resolution);
      const earlierThan = new Date(to * 1000);

      queryClient
        .fetchQuery({
          queryKey: ["perpsCandles", pair.id, earlierThan, interval],
          queryFn: () =>
            client.queryPerpsCandles({
              pairId: pair.id,
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
      const pair = pairFromSymbolInfo(symbolInfo);
      if (!pair) return;

      const interval = convertResolutionToCandleInterval(resolution);
      unsubscribe();

      _unsubscribe = subscriptions.subscribe("perpsCandles", {
        params: { pairId: pair.id, interval },
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
      const accountAddress = getAccountAddress();
      const pair = pairFromSymbolInfo(symbolInfo);
      if (!accountAddress || !pair) {
        onDataCallback([]);
        return;
      }

      fetchFillMarkers({
        client,
        queryClient,
        accountAddress,
        pairId: pair.id,
        resolution,
        from,
        to,
      })
        .then(onDataCallback)
        .catch((error: unknown) => {
          console.error("Failed to fetch perps fill markers:", error);
          onDataCallback([]);
        });
    },

    searchSymbols: () => {},
    unsubscribeBars: () => {
      unsubscribe();
    },
  };
}
