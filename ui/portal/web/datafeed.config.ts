import { Decimal } from "@left-curve/dango/utils";
import { CandleInterval } from "@left-curve/dango/types";

import type { Candle, CandleIntervals, PublicClient } from "@left-curve/dango/types";
import type { useConfig } from "@left-curve/store";
import type { AnyCoin } from "@left-curve/store/types";
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

type CreateDataFeedParameters = {
  client: PublicClient;
  queryClient: QueryClient;
  subscriptions: ReturnType<typeof useConfig>["subscriptions"];
  coins: Record<string, AnyCoin>;
};

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

function candlesToTradingViewBar(candles: Candle[], baseCoin: AnyCoin, quoteCoin: AnyCoin) {
  return candles.reverse().map((candle) => ({
    time: candle.timeStartUnix,
    volume: +Decimal(candle.volumeQuote).div(Decimal(10).pow(quoteCoin.decimals)).toFixed(5),
    open: +Decimal(candle.open)
      .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
      .toFixed(5),
    high: +Decimal(candle.high)
      .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
      .toFixed(5),
    low: +Decimal(candle.low)
      .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
      .toFixed(5),
    close: +Decimal(candle.close)
      .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
      .toFixed(5),
  }));
}

export function createTradingViewDataFeed(parameters: CreateDataFeedParameters): IBasicDataFeed {
  const { client, queryClient, subscriptions, coins } = parameters;

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
      onResolveErrorCallback: (reason: string) => void,
      _extension?: unknown,
    ) => {
      const [baseSymbol, quoteSymbol] = symbolName.split("-");
      const baseCoin = coins[baseSymbol];
      const quoteCoin = coins[quoteSymbol];

      if (!baseCoin || !quoteCoin) {
        return onResolveErrorCallback("Pair not found");
      }

      const symbolInfo: LibrarySymbolInfo = {
        name: symbolName,
        ticker: symbolName,
        description: `${baseCoin.symbol} / ${quoteCoin.symbol}`,
        session: "24x7",
        type: "crypto",
        timezone: "Etc/UTC",
        exchange: "Dango",
        listed_exchange: "Dango",
        format: "price",
        pricescale: 10 ** quoteCoin.decimals,
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
      const [baseSymbol, quoteSymbol] = symbolInfo.name.split("-");
      const baseCoin = coins[baseSymbol];
      const quoteCoin = coins[quoteSymbol];
      const interval = convertResolutionToCandleInterval(resolution);

      const earlierThan = new Date(to * 1000);

      queryClient
        .fetchQuery({
          queryKey: ["candles", symbolInfo.name, earlierThan, interval],
          queryFn: () =>
            client.queryCandles({
              baseDenom: baseCoin.denom,
              quoteDenom: quoteCoin.denom,
              interval,
              earlierThan: earlierThan.toJSON(),
            }),
        })
        .then(({ nodes }) => {
          const bars = candlesToTradingViewBar(nodes, baseCoin, quoteCoin);

          if (bars.length > 0) {
            onHistoryCallback(bars, { noData: false });
          } else {
            onHistoryCallback([], { noData: true });
          }
        })
        .catch((error: any) => {
          console.error("Failed to fetch bars:", error);
          onErrorCallback(error?.message || String(error));
        });
    },

    subscribeBars: (
      symbolInfo: LibrarySymbolInfo,
      resolution: ResolutionString,
      onRealtimeCallback: SubscribeBarsCallback,
      _subscriberId: string,
    ) => {
      const [baseSymbol, quoteSymbol] = symbolInfo.name.split("-");
      const baseCoin = coins[baseSymbol];
      const quoteCoin = coins[quoteSymbol];
      const interval = convertResolutionToCandleInterval(resolution);
      unsubscribe();

      _unsubscribe = subscriptions.subscribe("candles", {
        params: { baseDenom: baseCoin.denom, quoteDenom: quoteCoin.denom, interval },
        listener: ({ candles }) => {
          if (candles.length > 0) {
            const [newBar] = candlesToTradingViewBar(candles, baseCoin, quoteCoin);
            onRealtimeCallback(newBar);
          }
        },
      });
    },

    searchSymbols: () => {},
    unsubscribeBars: () => {},
  };
}
