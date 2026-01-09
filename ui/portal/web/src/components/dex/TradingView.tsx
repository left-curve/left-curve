import type React from "react";
import { useEffect, useRef } from "react";
import { useApp, useTheme } from "@left-curve/applets-kit";
import { useConfig, usePublicClient, useStorage } from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";

import * as TV from "@left-curve/tradingview";
import { createTradingViewDataFeed } from "~/datafeed";
import { Direction } from "@left-curve/dango/types";
import { Decimal, adjustPrice } from "@left-curve/dango/utils";

import type { AnyCoin } from "@left-curve/store/types";
import type { OrdersByUserResponse, WithId } from "@left-curve/dango/types";

type TradingViewProps = {
  coins: { base: AnyCoin; quote: AnyCoin };
  orders: WithId<OrdersByUserResponse>[];
};

export const TradingView: React.FC<TradingViewProps> = ({ coins, orders }) => {
  const pairSymbol = `${coins.base.symbol}-${coins.quote.symbol}`;

  const { theme } = useTheme();
  const publicClient = usePublicClient();
  const queryClient = useQueryClient();
  const { subscriptions, settings } = useApp();
  const { coins: allCoins } = useConfig();
  const { base, quote } = coins;
  const { timeFormat, timeZone } = settings;

  const [chartState, setChartState, hasLoaded] = useStorage<object>(`tradingview.${pairSymbol}`, {
    sync: true,
    version: 1.2,
    migrations: {
      "*": () => ({}),
    },
  });

  const widgetRef = useRef<TV.IChartingLibraryWidget | null>(null);

  useEffect(() => {
    if (!hasLoaded) return;

    localStorage.setItem(
      "tradingview.time_hours_format",
      timeFormat.includes("a") ? "12-hours" : "24-hours",
    );

    const toolbar_bg = theme === "dark" ? "#2d2c2a" : "#FFFCF6";
    const textColor = theme === "dark" ? "#FFFCF6" : "#2E2521";

    const datafeed = createTradingViewDataFeed({
      client: publicClient,
      queryClient,
      subscriptions,
      coins: allCoins.bySymbol,
    });

    const widget = new TV.widget({
      container: "tv-container",
      autosize: true,
      symbol: pairSymbol,
      interval: "5" as TV.ResolutionString,
      locale: "en",
      library_path: "/static/charting_library/",
      custom_css_url: "/styles/tv-overrides.css",
      theme,
      auto_save_delay: 1,
      datafeed,
      loading_screen: {
        backgroundColor: "transparent",
        foregroundColor: "#F9A9B2",
      },
      time_frames: [],
      enabled_features: ["seconds_resolution"],
      disabled_features: [
        "legend_inplace_edit",
        "display_market_status",
        "header_symbol_search",
        "header_compare",
        "header_saveload",
        "symbol_search_hot_key",
        "symbol_info",
        "go_to_date",
        "header_layouttoggle",
        "trading_account_manager",
        "create_volume_indicator_by_default",
      ],
      saved_data: chartState ? chartState : undefined,
      overrides: {
        "mainSeriesProperties.candleStyle.upColor": "#27AE60",
        "mainSeriesProperties.candleStyle.downColor": "#EB5757",
        "mainSeriesProperties.candleStyle.borderUpColor": "#27AE60",
        "mainSeriesProperties.candleStyle.borderDownColor": "#EB5757",
        "mainSeriesProperties.candleStyle.wickUpColor": "#27AE60",
        "mainSeriesProperties.candleStyle.wickDownColor": "#EB5757",
        "paneProperties.backgroundType": "solid",
        "paneProperties.background": toolbar_bg,
        "paneProperties.topMargin": 10,
        "paneProperties.bottomMargin": 10,
      },

      studies_overrides: {
        "volume.volume.color.0": "#EB5757",
        "volume.volume.color.1": "#27AE60",
        "volume.volume.transparency": 50,
      },
    });

    const saveFn = () => widget.save(setChartState);

    const invalidateCandles = () => {
      widget.resetCache();
      widget.chart().resetData();
    };

    publicClient.subscribe?.emitter?.addListener("connected", invalidateCandles);

    widget.onChartReady(() => {
      widgetRef.current = widget;
      const chart = widget.chart();
      const allStudies = chart.getAllStudies();

      const volumeExists = allStudies.some((study) => study.name === "Volume");
      if (!volumeExists) chart.createStudy("Volume", false, false);

      widget.subscribe("onAutoSaveNeeded", saveFn);
      widget.applyOverrides({
        "paneProperties.background": toolbar_bg,
        "scalesProperties.textColor": textColor,
        timezone:
          timeZone === "utc"
            ? "Etc/UTC"
            : (Intl.DateTimeFormat().resolvedOptions().timeZone as TV.TimezoneId),
      });
    });
    return () => {
      publicClient.subscribe?.emitter?.removeListener("connected", invalidateCandles);
      widgetRef.current?.remove();
      widgetRef.current = null;
    };
  }, [theme, hasLoaded]);

  useEffect(() => {
    if (!widgetRef.current) return;
    const chart = widgetRef.current.chart();
    if (chart.symbol() !== pairSymbol) {
      chart.setSymbol(pairSymbol, () => {});
    }
  }, [coins]);

  useEffect(() => {
    if (!widgetRef.current) return;

    const chart = widgetRef.current.chart();

    chart.getAllShapes().forEach((shape) => chart.removeEntity(shape.id));
    orders.forEach((order) => {
      const price = adjustPrice(
        +Decimal(order.price)
          .times(Decimal(10).pow(base.decimals - quote.decimals))
          .toFixed(),
      );

      const color = order.direction === Direction.Buy ? "#27AE60" : "#EB5757";

      chart.createShape(
        { price: +price, time: Date.now() },
        {
          shape: "horizontal_line",
          lock: true,
          disableSelection: true,
          overrides: {
            showLabel: true,
            textcolor: color,
            linecolor: color,
            linestyle: 2,
            linewidth: 1,
            bodybgcolor: color,
          },
        },
      );
    });
  }, [orders]);

  return <div id="tv-container" className="w-full lg:min-h-[32.875rem] h-full" />;
};
