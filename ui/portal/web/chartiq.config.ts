import getDefaultConfig from "@left-curve/chartiq/js/defaultConfiguration";
import { CandleInterval } from "@left-curve/dango/types";

import { CIQ } from "@left-curve/chartiq";
import getLicenseKey from "@left-curve/chartiq/license/key";
import { Decimal } from "@left-curve/dango/utils";

getLicenseKey(CIQ);

import type { Candle, CandleIntervals, PublicClient } from "@left-curve/dango/types";
import type { useConfig } from "@left-curve/store";
import type { AnyCoin } from "@left-curve/store/types";

type CreateChartIQDataFeedParameters = {
  client: PublicClient;
  subscriptions: ReturnType<typeof useConfig>["subscriptions"];
  coins: AnyCoin[];
  updateChartData: (
    appendQuotes: CIQ.ChartEngine.OHLCQuote[] | CIQ.ChartEngine.LastSale,
    chart?: CIQ.ChartEngine.Chart,
    params?: {
      noCreateDataSet?: boolean;
      noCleanupDates?: boolean;
      allowReplaceOHL?: boolean;
      bypassGovernor?: boolean;
      fillGaps?: boolean;
      secondarySeries?: string;
      deleteItems?: boolean;
      useAsLastSale?: {
        aggregatedVolume?: boolean;
      };
    },
  ) => void;
};

export function createChartIQDataFeed(parameters: CreateChartIQDataFeedParameters) {
  const { client, subscriptions, coins, updateChartData } = parameters;

  let _unsubscribe: () => void = () => {};

  const unsubscribe: () => void = () => {
    _unsubscribe();
  };

  let context: CIQ.ChartEngine;

  const coinsBySymbol = Object.fromEntries(Object.values(coins).map((coin) => [coin.symbol, coin]));

  type FetchInitialDataCallback = (params: {
    quotes: any[];
    moreAvailable: boolean;
    attribution: { source: string; exchange: string };
  }) => void;

  async function queryCandles(
    pairSymbol: string,
    startDate: Date,
    endDate: Date,
    params: { stx: CIQ.ChartEngine; symbol: string },
  ) {
    const [baseSymbol, quoteSymbol] = pairSymbol.split("-");
    const baseCoin = coinsBySymbol[baseSymbol];
    const quoteCoin = coinsBySymbol[quoteSymbol];
    const { periodicity: period, interval, timeUnit } = params.stx.layout;

    const candleInterval = convertPeriodicityToCandleInterval({
      interval: interval,
      period,
      timeUnit: timeUnit,
    });

    const { nodes } = await client.queryCandles({
      baseDenom: baseCoin.denom,
      quoteDenom: quoteCoin.denom,
      interval: candleInterval,
      laterThan: startDate.toJSON(),
      earlierThan: endDate.toJSON(),
    });

    return candlesToChartIQData(nodes);
  }

  async function fetchInitialData(
    pairSymbol: string,
    startDate: Date,
    endDate: Date,
    params: { stx: CIQ.ChartEngine; symbol: string },
    cb: FetchInitialDataCallback,
  ) {
    const quotes = await queryCandles(pairSymbol, startDate, endDate, params);

    cb({
      quotes,
      moreAvailable: quotes.length === 100,
      attribution: { source: "dango", exchange: "REAL-TIME" },
    });
  }

  async function fetchPaginationData(
    pairSymbol: string,
    startDate: Date,
    endDate: Date,
    params: { stx: CIQ.ChartEngine; symbol: string },
    cb: FetchInitialDataCallback,
  ) {
    const quotes = await queryCandles(pairSymbol, startDate, endDate, params);
    cb({
      quotes: quotes,
      moreAvailable: !!(quotes.length - 1),
      attribution: { source: "dango", exchange: "REAL-TIME" },
    });
  }

  function candlesToChartIQData(candles: Candle[]) {
    return candles.reverse().map((candle) => ({
      Volume: +Decimal(candle.volumeQuote).div(Decimal(10).pow(6)).toFixed(0, 0),
      DT: new Date(candle.timeStart),
      Open: +candle.open,
      High: +candle.high,
      Low: +candle.low,
      Close: +candle.close,
    }));
  }

  function subscribe(params: { stx: CIQ.ChartEngine; symbol: string }) {
    const { symbol, stx } = params;
    const { periodicity: period, interval, timeUnit } = stx.layout;

    const [baseSymbol, quoteSymbol] = symbol.split("-");
    const baseCoin = coinsBySymbol[baseSymbol];
    const quoteCoin = coinsBySymbol[quoteSymbol];

    const candleInterval = convertPeriodicityToCandleInterval({
      interval: interval,
      period,
      timeUnit: timeUnit,
    });
    _unsubscribe = subscriptions.subscribe("candles", {
      params: {
        baseDenom: baseCoin.denom,
        quoteDenom: quoteCoin.denom,
        interval: candleInterval,
        limit: 1,
      },
      listener: ({ candles }) => {
        const chartData = candlesToChartIQData(candles);
        context?.updateChartData(chartData);
        updateChartData(chartData);
      },
    });
  }

  function setStx(stx: CIQ.ChartEngine) {
    context = stx;
  }

  return {
    fetchInitialData,
    fetchPaginationData,
    subscribe,
    unsubscribe,
    setStx,
  };
}

type ChartIQPeriodicity = {
  period: number;
  interval: number | "day" | "week" | "month" | string;
  timeUnit: "second" | "minute" | "day" | "week" | "month" | string;
};

function convertPeriodicityToCandleInterval(periodicity: ChartIQPeriodicity): CandleIntervals {
  const { period, interval, timeUnit } = periodicity;

  const unit = typeof interval === "string" ? interval : timeUnit;
  const multiplier = typeof interval === "number" ? interval : 1;

  const totalDuration = period * multiplier;

  switch (unit) {
    case "second":
      if (totalDuration === 1) return CandleInterval.OneSecond;
      break;

    case "minute":
      switch (totalDuration) {
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
      }
      break;

    case "day":
      if (totalDuration === 1) return CandleInterval.OneDay;
      break;

    case "week":
      if (totalDuration === 1) return CandleInterval.OneWeek;
      break;
  }

  throw new Error(`Unsupported ChartIQ periodicity: ${JSON.stringify(periodicity)}`);
}

type CreateChartIQConfigParameters = {
  pairSymbol: string;
  dataFeed: ReturnType<typeof createChartIQDataFeed>;
  theme: "light" | "dark";
};
export function createChartIQConfig(params: CreateChartIQConfigParameters) {
  const { pairSymbol, dataFeed, theme } = params;
  const config = getDefaultConfig({});

  config.attributions = {
    sources: {
      dango: '<a target="_blank" href="https://dango.exchange/">Dango</a>',
    },
    exchanges: {
      "REAL-TIME": "Data is real-time.",
    },
  };

  config.initialSymbol = {
    symbol: pairSymbol,
    name: pairSymbol,
    exchDisp: "Dango",
  };

  config.themes.defaultTheme = theme === "light" ? "ciq-day" : "ciq-night";

  config.chartEngineParams = {
    preferences: {
      ...config.chartEngineParams?.preferences,
      currentPriceLine: true,
      whitespace: 0,
    },
    // @ts-ignore
    chart: {
      yAxis: {
        position: "right",
      },
    },
  };

  config.quoteFeeds = [
    {
      quoteFeed: dataFeed,
      // @ts-ignore
      behavior: { refreshInterval: 0 },
    },
  ];

  config.menus.markers = {
    content: [
      {
        type: "heading",
        label: "SignalIQ",
        feature: "signaliq",
        menuPersist: true,
      },
      {
        type: "heading",
        label: "Chart Events",
        menuPersist: true,
      },
      {
        type: "switch",
        label: "Orders",
        setget: "Markers.MarkerSwitch",
        value: "square",
      },
    ],
  };

  config.menus.preferences = {
    content: [
      {
        type: "heading",
        label: "Chart Preferences",
        menuPersist: true,
      },
      {
        type: "switch",
        label: "Range Selector",
        setget: "Layout.RangeSlider",
        feature: "rangeslider",
        menuPersist: true,
      },
      {
        type: "switch",
        label: "Animation",
        setget: "Layout.Animation",
        feature: "animation",
        menuPersist: true,
      },
      {
        type: "switch",
        label: "Hide Outliers",
        setget: "Layout.Outliers",
        feature: "outliers",
        menuPersist: true,
      },
      {
        type: "switch",
        label: "Market Depth",
        setget: "Layout.MarketDepth",
        feature: "marketdepth",
        menuPersist: true,
      },
      {
        type: "switch",
        label: "L2 Heat Map",
        setget: "Layout.L2Heatmap",
        feature: "marketdepth",
        menuPersist: true,
      },
      {
        type: "separator",
        menuPersist: true,
      },
      {
        type: "heading",
        label: "Y-Axis Preferences",
        menuPersist: true,
      },
      {
        type: "switch",
        label: "Log Scale",
        setget: "Layout.ChartScale",
        value: "log",
        menuPersist: true,
      },
      {
        type: "switch",
        label: "Invert",
        setget: "Layout.FlippedChart",
        menuPersist: true,
      },
      {
        type: "separator",
        menuPersist: true,
      },
      {
        type: "heading",
        label: "Chart Preferences",
        menuPersist: true,
      },
      {
        type: "radio",
        label: "Hide Heads-Up Display",
        setget: "Layout.HeadsUp",
        value: "crosshair",
      },
      {
        type: "radio",
        label: "Show Heads-Up Display",
        setget: "Layout.HeadsUp",
        value: "static",
      },
      {
        type: "separator",
        menuPersist: true,
      },
      /*        {
                type: "item",
                label: "Shortcuts / Hotkeys",
                tap: "Layout.showShortcuts",
                value: true,
                feature: "shortcuts",
              }, */
      {
        type: "heading",
        label: "Locale",
        menuPersist: true,
      },
      {
        type: "clickable",
        label: "Change Timezone",
        // @ts-ignore
        selector: "cq-timezone-dialog",
        method: "open",
      },
      {
        type: "item",
        label: "Change Language",
        setget: "Layout.Language",
        iconCls: "flag",
      },
    ],
  };

  config.menus.period = {
    content: [
      {
        type: "item",
        label: "1 D",
        tap: "Layout.setPeriodicity",
        value: [1, 1, "day"] as string[],
      },
      {
        type: "item",
        label: "1 W",
        tap: "Layout.setPeriodicity",
        value: [1, 1, "week"] as string[],
      },
      {
        type: "separator",
        menuPersist: true,
      },
      {
        type: "item",
        label: "1 Min",
        tap: "Layout.setPeriodicity",
        value: [1, 1, "minute"] as string[],
      },
      {
        type: "item",
        label: "5 Min",
        tap: "Layout.setPeriodicity",
        value: [1, 5, "minute"] as string[],
      },
      {
        type: "item",
        label: "15 Min",
        tap: "Layout.setPeriodicity",
        value: [3, 5, "minute"] as string[],
      },
      {
        type: "item",
        label: "1 Hour",
        tap: "Layout.setPeriodicity",
        value: [2, 30, "minute"] as string[],
      },
      {
        type: "item",
        label: "4 Hour",
        tap: "Layout.setPeriodicity",
        value: [8, 30, "minute"] as string[],
      },
      {
        type: "separator",
        menuPersist: true,
      },
      {
        type: "item",
        label: "1 Sec",
        tap: "Layout.setPeriodicity",
        value: [1, 1, "second"] as string[],
      },
    ],
  };

  return config;
}
