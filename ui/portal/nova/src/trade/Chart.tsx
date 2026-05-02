import { useEffect, useRef } from "react";
import { View } from "react-native";
import { useConfig, usePublicClient } from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";
import * as TV from "@left-curve/tradingview";
import { createPerpsDataFeed } from "~/datafeed";
import { useNovaTheme } from "../layout/useNovaTheme";
import { Card } from "../components";

const CANDLE_UP = "#4CAF50";
const CANDLE_DOWN = "#E57373";

function getThemeColors(mode: "light" | "dark") {
  return mode === "dark"
    ? {
        background: "#1f1a16",
        text: "#f4ebdc",
        grid: "#f4ebdc0F",
        separator: "#5a5247",
        crosshairLabel: "#f4ebdc",
        axisHighlight: "rgba(244, 235, 220, 0.25)",
        accent: "#e0b47a",
      }
    : {
        background: "#fffcf5",
        text: "#1f1a16",
        grid: "#1f1a160A",
        separator: "#b0a89e",
        crosshairLabel: "#1f1a16",
        axisHighlight: "rgba(31, 26, 22, 0.25)",
        accent: "#c9a26b",
      };
}

export function Chart() {
  const { mode } = useNovaTheme();
  const publicClient = usePublicClient();
  const queryClient = useQueryClient();
  const { subscriptions } = useConfig();

  const widgetRef = useRef<TV.IChartingLibraryWidget | null>(null);

  const pairSymbol = "ETH-USD";
  const storageKey = `nova_tv.${pairSymbol}`;

  useEffect(() => {
    const tvTheme = mode === "dark" ? "dark" : "light";
    const colors = getThemeColors(mode);

    const datafeed = createPerpsDataFeed({
      client: publicClient,
      queryClient,
      subscriptions,
    });

    const widget = new TV.widget({
      container: "nova-tv-container",
      autosize: true,
      symbol: pairSymbol,
      interval: "15" as TV.ResolutionString,
      locale: "en",
      library_path: "/static/charting_library/",
      custom_css_url: "/styles/nova-tv-overrides.css",
      theme: tvTheme,
      auto_save_delay: 1,
      datafeed,
      loading_screen: {
        backgroundColor: "transparent",
        foregroundColor: colors.accent,
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
        "mainSeriesProperties.candleStyle.upColor": CANDLE_UP,
        "mainSeriesProperties.candleStyle.downColor": CANDLE_DOWN,
        "mainSeriesProperties.candleStyle.borderUpColor": CANDLE_UP,
        "mainSeriesProperties.candleStyle.borderDownColor": CANDLE_DOWN,
        "mainSeriesProperties.candleStyle.wickUpColor": CANDLE_UP,
        "mainSeriesProperties.candleStyle.wickDownColor": CANDLE_DOWN,
        "paneProperties.backgroundType": "solid",
        "paneProperties.background": colors.background,
        "paneProperties.separatorColor": colors.separator,
        "paneProperties.topMargin": 10,
        "paneProperties.bottomMargin": 10,
        "paneProperties.vertGridProperties.color": colors.grid,
        "paneProperties.horzGridProperties.color": colors.grid,
        "scalesProperties.crosshairLabelBgColorLight": "#1f1a16",
        "scalesProperties.crosshairLabelBgColorDark": "#f4ebdc",
        "scalesProperties.axisHighlightColor": colors.axisHighlight,
        "scalesProperties.axisLineToolLabelBackgroundColorCommon": colors.text,
        "scalesProperties.axisLineToolLabelBackgroundColorActive": colors.text,
      },
      settings_overrides: {
        "linetooltrendline.linecolor": colors.text,
        "linetooltrendline.textcolor": colors.text,
        "linetoolhorzline.linecolor": colors.text,
        "linetoolhorzline.textcolor": colors.text,
        "linetoolhorzray.linecolor": colors.text,
        "linetoolhorzray.textcolor": colors.text,
        "linetoolray.linecolor": colors.text,
        "linetoolray.textcolor": colors.text,
        "linetoolextended.linecolor": colors.text,
        "linetoolextended.textcolor": colors.text,
        "linetoolarrow.linecolor": colors.text,
        "linetoolarrow.textcolor": colors.text,
        "linetoolcrossline.linecolor": colors.text,
        "linetoolbezierquadro.linecolor": colors.text,
      },
      studies_overrides: {
        "volume.volume.color.0": CANDLE_DOWN,
        "volume.volume.color.1": CANDLE_UP,
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

      if (!allStudies.some((study) => study.name === "Volume")) {
        chart.createStudy("Volume", false, false);
      }

      widget.subscribe("onAutoSaveNeeded", saveFn);
      widget.applyOverrides({
        "paneProperties.background": colors.background,
        "paneProperties.separatorColor": colors.separator,
        "scalesProperties.textColor": colors.text,
        "paneProperties.vertGridProperties.color": colors.grid,
        "paneProperties.horzGridProperties.color": colors.grid,
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone as TV.TimezoneId,
      });
    });

    return () => {
      publicClient.subscribe?.emitter?.removeListener("connected", invalidateCandles);
      widgetRef.current?.remove();
      widgetRef.current = null;
    };
  }, [mode]);

  return (
    <Card className="flex flex-col overflow-hidden h-full">
      <View className="flex-1 min-h-0">
        <div id="nova-tv-container" className="w-full h-full" />
      </View>
    </Card>
  );
}
