import type React from "react";
import { useEffect, useRef } from "react";
import { useTheme } from "@left-curve/applets-kit";
import { useApp } from "@left-curve/foundation";
import {
  useConfig,
  usePublicClient,
  usePerpsUserStateExtended,
  usePerpsOrdersByUser,
} from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";

import * as TV from "@left-curve/tradingview";
import { deepEqual } from "@left-curve/utils";
import { createPerpsDataFeed } from "~/datafeed";
import { buildPositionLines, buildPerpsOrderLines, drawLines } from "../helpers/chartLines";

import type { AnyCoin } from "@left-curve/store/types";

type TradingViewProps = {
  coins: { base: AnyCoin; quote: AnyCoin };
  perpsPairId: string;
  accountAddress?: string;
};

export const TradingView: React.FC<TradingViewProps> = ({ coins, perpsPairId, accountAddress }) => {
  const pairSymbol = `${coins.base.symbol}-USD`;

  const position = usePerpsUserStateExtended((s) => s.positions[perpsPairId], { accountAddress });
  const perpsOrders = usePerpsOrdersByUser(
    (s) => {
      if (!s.orders) return null;
      return Object.fromEntries(
        Object.entries(s.orders).filter(([, order]) => order.pairId === perpsPairId),
      );
    },
    { accountAddress },
    deepEqual,
  );

  const { theme } = useTheme();
  const publicClient = usePublicClient();
  const queryClient = useQueryClient();
  const { subscriptions } = useConfig();
  const timeFormat = useApp((state) => state.settings.timeFormat);
  const timeZone = useApp((state) => state.settings.timeZone);

  const storageKey = `tv_v4.${pairSymbol}_perps`;

  const widgetRef = useRef<TV.IChartingLibraryWidget | null>(null);
  const readyRef = useRef(false);
  // The datafeed is created with the widget; this keeps getMarks pointed at the live account.
  const accountAddressRef = useRef(accountAddress);
  accountAddressRef.current = accountAddress;

  useEffect(() => {
    try {
      localStorage.setItem(
        "tradingview.time_hours_format",
        timeFormat.includes("a") ? "12-hours" : "24-hours",
      );
    } catch {}

    const toolbar_bg = theme === "dark" ? "#2d2c2a" : "#FFFCF6";
    const textColor = theme === "dark" ? "#FFFCF6" : "#2E2521";

    const datafeed = createPerpsDataFeed({
      client: publicClient,
      queryClient,
      subscriptions,
      getAccountAddress: () => accountAddressRef.current,
    });

    const widget = new TV.widget({
      container: "tv-container",
      autosize: true,
      symbol: pairSymbol,
      interval: "5" as TV.ResolutionString,
      locale: "en",
      library_path: `/charting_library/${import.meta.env.TV_VERSION}/`,
      custom_css_url: `${window.location.origin}/styles/tv-overrides.css`,
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
        "scalesProperties.crosshairLabelBgColorLight": "#2E2521",
        "scalesProperties.crosshairLabelBgColorDark": "#FFFCF6",
        "scalesProperties.axisHighlightColor":
          theme === "dark" ? "rgba(255, 252, 246, 0.25)" : "rgba(46, 37, 33, 0.25)",
        "scalesProperties.axisLineToolLabelBackgroundColorCommon": textColor,
        "scalesProperties.axisLineToolLabelBackgroundColorActive": textColor,
        ...(theme === "dark" && {
          "paneProperties.vertGridProperties.color": "#ffffff0F",
          "paneProperties.horzGridProperties.color": "#ffffff0F",
        }),
      },

      settings_overrides: {
        "linetooltrendline.linecolor": textColor,
        "linetooltrendline.textcolor": textColor,
        "linetoolhorzline.linecolor": textColor,
        "linetoolhorzline.textcolor": textColor,
        "linetoolhorzray.linecolor": textColor,
        "linetoolhorzray.textcolor": textColor,
        "linetoolray.linecolor": textColor,
        "linetoolray.textcolor": textColor,
        "linetoolextended.linecolor": textColor,
        "linetoolextended.textcolor": textColor,
        "linetoolarrow.linecolor": textColor,
        "linetoolarrow.textcolor": textColor,
        "linetoolcrossline.linecolor": textColor,
        "linetoolbezierquadro.linecolor": textColor,
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
      if (!widgetRef.current || !readyRef.current) return;
      try {
        widgetRef.current.resetCache();
        widgetRef.current.chart().resetData();
      } catch {
        // Iframe not yet same-origin or widget torn down — next getBars will refetch.
      }
    };

    widget.onChartReady(() => {
      widgetRef.current = widget;
      readyRef.current = true;
      publicClient.subscribe?.emitter?.on("connected", invalidateCandles);
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
      readyRef.current = false;
      publicClient.subscribe?.emitter?.off("connected", invalidateCandles);
      widget.remove();
      widgetRef.current = null;
    };
  }, [theme]);

  useEffect(() => {
    if (!widgetRef.current) return;
    const chart = widgetRef.current.chart();

    const syncMarks = () => {
      if (accountAddress) chart.refreshMarks();
      else chart.clearMarks();
    };

    if (chart.symbol() !== pairSymbol) {
      chart.setSymbol(pairSymbol, syncMarks);
      return;
    }

    syncMarks();
  }, [accountAddress, pairSymbol]);

  useEffect(() => {
    if (!widgetRef.current) return;
    const chart = widgetRef.current.chart();

    const lines = [
      ...(position ? buildPositionLines(position) : []),
      ...(perpsOrders ? buildPerpsOrderLines(perpsOrders, perpsPairId) : []),
    ];

    drawLines(chart, lines);
  }, [position, perpsOrders, perpsPairId]);

  return <div id="tv-container" className="w-full lg:min-h-[32.875rem] h-full" />;
};
