import getDefaultConfig from "@left-curve/chartiq/js/defaultConfiguration";
import { CandleInterval } from "@left-curve/dango/types";

import { CIQ } from "@left-curve/chartiq";
import getLicenseKey from "@left-curve/chartiq/license/key";
import { Decimal } from "@left-curve/dango/utils";

getLicenseKey(CIQ);
createChartIQUIOverride();

import type { Candle, CandleIntervals, PublicClient } from "@left-curve/dango/types";
import type { useConfig } from "@left-curve/store";
import type { AnyCoin } from "@left-curve/store/types";
import type { QueryClient } from "@tanstack/react-query";

type CreateChartIQDataFeedParameters = {
  client: PublicClient;
  queryClient: QueryClient;
  subscriptions: ReturnType<typeof useConfig>["subscriptions"];
  coins: Record<string, AnyCoin>;
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
  const { client, queryClient, subscriptions, coins, updateChartData } = parameters;

  let _unsubscribe: () => void = () => {};

  const unsubscribe: () => void = () => {
    _unsubscribe();
  };

  let context: CIQ.ChartEngine;

  type FetchInitialDataCallback = (params: {
    quotes: any[];
    moreAvailable: boolean;
    attribution: { source: string; exchange: string };
  }) => void;

  async function queryCandles(
    pairSymbol: string,
    _startDate: Date,
    endDate: Date,
    params: { stx: CIQ.ChartEngine; symbol: string },
  ) {
    const [baseSymbol, quoteSymbol] = pairSymbol.split("-");
    const baseCoin = coins[baseSymbol];
    const quoteCoin = coins[quoteSymbol];
    const { periodicity: period, interval, timeUnit } = params.stx.layout;

    const candleInterval = convertPeriodicityToCandleInterval({
      interval: interval,
      period,
      timeUnit: timeUnit,
    });

    const date = new Date(endDate);
    date.setMilliseconds(0);

    const { nodes } = await queryClient.fetchQuery({
      queryKey: ["candles", pairSymbol, date.toJSON(), candleInterval],
      queryFn: () =>
        client.queryCandles({
          baseDenom: baseCoin.denom,
          quoteDenom: quoteCoin.denom,
          interval: candleInterval,
          earlierThan: date.toJSON(),
        }),
    });

    return candlesToChartIQData(nodes, baseCoin, quoteCoin);
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

  function candlesToChartIQData(candles: Candle[], baseCoin: AnyCoin, quoteCoin: AnyCoin) {
    return candles.reverse().map((candle) => ({
      Volume: +Decimal(candle.volumeQuote).div(Decimal(10).pow(6)).toFixed(0, 0),
      DT: new Date(candle.timeStart),
      Open: +Decimal(candle.open)
        .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
        .toFixed(),
      High: +Decimal(candle.high)
        .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
        .toFixed(),
      Low: +Decimal(candle.low)
        .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
        .toFixed(),
      Close: +Decimal(candle.close)
        .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
        .toFixed(),
    }));
  }

  function subscribe(params: { stx: CIQ.ChartEngine; symbol: string }) {
    const { symbol, stx } = params;
    const { periodicity: period, interval, timeUnit } = stx.layout;

    const [baseSymbol, quoteSymbol] = symbol.split("-");
    const baseCoin = coins[baseSymbol];
    const quoteCoin = coins[quoteSymbol];

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
      },
      listener: ({ candles }) => {
        const chartData = candlesToChartIQData(candles, baseCoin, quoteCoin);
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

  config.chartId = pairSymbol;

  config.restore = false;

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
    layout: {
      periodicity: 1,
      interval: 5,
      timeUnit: "minute",
    },
    // @ts-ignore
    chart: {
      layout: {
        periodicity: 1,
        interval: 1,
        timeUnit: "minute",
      },
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

function createChartIQUIOverride() {
  class CustomTitle extends CIQ.UI.components("cq-chart-title")[0].classDefinition {}

  CustomTitle.markup = `
    <cq-symbol class="hide-outline"></cq-symbol>

    <cq-menu
      class="ciq-period"
      config="period"
      reader="Periodicity"
      text
      binding="Layout.periodicity"
      title="Interval Selector"
      lift-dropdown
    ></cq-menu>

    <cq-chart-price>
      <span id="pricelabel" hidden>Current Price</span>

      <div role="group" aria-labelledby="pricelabel">
        <cq-current-price role="marquee" cq-animate></cq-current-price>
      </div>

      <span>
        <span id="changelabel" hidden>Change</span>

        <div role="group" aria-labelledby="changelabel">
          <div class="ciq-screen-reader" accessiblechange role="marquee"></div>
        </div>
        <cq-change aria-hidden="true">
          <div class="ico"></div>
          <cq-todays-change></cq-todays-change>
          <cq-todays-change-pct></cq-todays-change-pct>
        </cq-change>
      </span>
    </cq-chart-price>
    <div class="exchange"></div>
  `;

  CIQ.UI.addComponentDefinition("cq-chart-title", CustomTitle);
}
