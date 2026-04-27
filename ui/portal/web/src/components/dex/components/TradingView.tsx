import type React from "react";
import { useEffect, useRef } from "react";
import { useApp, useTheme } from "@left-curve/applets-kit";
import {
  useConfig,
  usePublicClient,
  perpsUserStateExtendedStore,
  perpsOrdersByUserStore,
} from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";

import * as TV from "@left-curve/tradingview";
import { createTradingViewDataFeed, createPerpsDataFeed } from "~/datafeed";
import {
  buildPositionLines,
  buildPerpsOrderLines,
  buildSpotOrderLines,
  drawLines,
} from "../lib/chartLines";

import type { AnyCoin } from "@left-curve/store/types";
import type { OrdersByUserResponse, WithId } from "@left-curve/dango/types";

type TradingViewProps = {
  coins: { base: AnyCoin; quote: AnyCoin };
  orders: WithId<OrdersByUserResponse>[];
  mode?: "spot" | "perps";
};

export const TradingView: React.FC<TradingViewProps> = ({ coins, orders, mode = "spot" }) => {
  const isPerps = mode === "perps";
  const pairSymbol = isPerps
    ? `${coins.base.symbol}-USD`
    : `${coins.base.symbol}-${coins.quote.symbol}`;
  const perpsPairId = isPerps ? `perp/${coins.base.symbol.toLowerCase()}usd` : "";

  const positions = perpsUserStateExtendedStore((s) => s.positions);
  const perpsOrders = perpsOrdersByUserStore((s) => s.orders);

  const { theme } = useTheme();
  const publicClient = usePublicClient();
  const queryClient = useQueryClient();
  const { subscriptions, settings } = useApp();
  const { coins: allCoins } = useConfig();
  const { base, quote } = coins;
  const { timeFormat, timeZone } = settings;

  const storageKey = `tv_v1.${pairSymbol}_${mode}`;

  const widgetRef = useRef<TV.IChartingLibraryWidget | null>(null);

  useEffect(() => {
    try {
      localStorage.setItem(
        "tradingview.time_hours_format",
        timeFormat.includes("a") ? "12-hours" : "24-hours",
      );
    } catch {}

    const toolbar_bg = theme === "dark" ? "#2d2c2a" : "#FFFCF6";
    const textColor = theme === "dark" ? "#FFFCF6" : "#2E2521";

    const datafeed = isPerps
      ? createPerpsDataFeed({
          client: publicClient,
          queryClient,
          subscriptions,
        })
      : createTradingViewDataFeed({
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
      enabled_features: ["seconds_resolution", "iframe_loading_same_origin"],
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
      saved_data: JSON.parse(localStorage.getItem(storageKey) || "null"),
      overrides: {
        "mainSeriesProperties.candleStyle.upColor": "#27AE60",
        "mainSeriesProperties.candleStyle.downColor": "#EB5757",
        "mainSeriesProperties.candleStyle.borderUpColor": "#27AE60",
        "mainSeriesProperties.candleStyle.borderDownColor": "#EB5757",
        "mainSeriesProperties.candleStyle.wickUpColor": "#27AE60",
        "mainSeriesProperties.candleStyle.wickDownColor": "#EB5757",
        "paneProperties.backgroundType": "solid",
        "paneProperties.background": toolbar_bg,
        "paneProperties.separatorColor": theme === "dark" ? "#666360" : "#CCC7C0",
        "paneProperties.topMargin": 10,
        "paneProperties.bottomMargin": 10,
        "mainSeriesProperties.priceLineColor": textColor,
        "scalesProperties.crosshairLabelBgColorLight": "#2E2521",
        "scalesProperties.crosshairLabelBgColorDark": "#FFFCF6",
        ...(theme === "dark" && {
          "paneProperties.vertGridProperties.color": "#ffffff0F",
          "paneProperties.horzGridProperties.color": "#ffffff0F",
        }),
      },

      studies_overrides: {
        "volume.volume.color.0": "#EB5757",
        "volume.volume.color.1": "#27AE60",
        "volume.volume.transparency": 50,
      },
    });

    const saveFn = () =>
      widget.save((state) => {
        try {
          localStorage.setItem(storageKey, JSON.stringify(state));
        } catch {}
      });

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
        "paneProperties.separatorColor": theme === "dark" ? "#666360" : "#CCC7C0",
        "scalesProperties.textColor": textColor,
        ...(theme === "dark" && {
          "paneProperties.vertGridProperties.color": "#ffffff0F",
          "paneProperties.horzGridProperties.color": "#ffffff0F",
        }),
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
  }, [theme, mode]);

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

    const position = positions[perpsPairId];
    const lines = isPerps
      ? [
          ...(position ? buildPositionLines(position) : []),
          ...(perpsOrders ? buildPerpsOrderLines(perpsOrders, perpsPairId) : []),
        ]
      : buildSpotOrderLines(orders, base, quote);

    drawLines(chart, lines);
  }, [orders, positions, perpsOrders, perpsPairId, isPerps]);

  return <div id="tv-container" className="w-full lg:min-h-[32.875rem] h-full" />;
};
