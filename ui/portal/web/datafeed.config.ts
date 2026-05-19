import { CandleInterval } from "@left-curve/types";

import type { CandleIntervals, PerpsCandle } from "@left-curve/types";
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
} from "@left-curve/tradingview";

function convertResolutionToCandleInterval(resolution: ResolutionString): CandleIntervals {
  if (resolution.includes("S")) return CandleInterval.OneSecond;
  if (resolution.includes("W")) return CandleInterval.OneWeek;
  if (resolution.includes("D")) return CandleInterval.OneDay;

  const minutes = parseInt(resolution);
  if (Number.isNaN(minutes)) throw new Error(`Unsupported resolution: ${resolution}`);

  switch (minutes) {
    case 1:
      return CandleInterval.OneMinute;
    case 5:
      return CandleInterval.FiveMinutes;
    case 15:
      return CandleInterval.FifteenMinutes;
    case 60:
      return CandleInterval.OneHour;
    case 240:
      return CandleInterval.FourHours;
    default:
      throw new Error(`Unsupported resolution in minutes: ${minutes}`);
  }
}

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
};

function perpsSymbolToPairId(symbolName: string): string {
  const [base] = symbolName.split("-");
  return `perp/${base.toLowerCase()}usd`;
}

export function createPerpsDataFeed(parameters: CreatePerpsDataFeedParameters): IBasicDataFeed {
  const { client, queryClient, subscriptions } = parameters;

  let _unsubscribe: () => void = () => {};

  const unsubscribe: () => void = () => {
    _unsubscribe?.();
  };

  return {
    onReady: (callback: OnReadyCallback) => {
      setTimeout(
        () =>
          callback({
            supported_resolutions: [
              "1S",
              "1",
              "5",
              "15",
              "60",
              "240",
              "1D",
              "1W",
            ] as ResolutionString[],
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
        supported_resolutions: [
          "1S",
          "1",
          "5",
          "15",
          "60",
          "240",
          "1D",
          "1W",
        ] as ResolutionString[],
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
        .catch((error: any) => {
          console.error("Failed to fetch perps bars:", error);
          onErrorCallback(error?.message || String(error));
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

    searchSymbols: () => {},
    unsubscribeBars: () => {},
  };
}
