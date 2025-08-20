import type React from "react";
import { useEffect, useMemo, useRef } from "react";
import { useApp } from "~/hooks/useApp";
import { useTheme } from "@left-curve/applets-kit";
import { useConfig, usePublicClient } from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";

import * as TV from "@left-curve/tradingview";
import { createTradingViewDataFeed } from "~/datafeed";

import type { AnyCoin } from "@left-curve/store/types";
import type { OrdersByUserResponse } from "@left-curve/dango/types";

type TradingViewProps = {
  coins: { base: AnyCoin; quote: AnyCoin };
  orders: OrdersByUserResponse[];
};

export const TradingView: React.FC<TradingViewProps> = ({ coins, orders }) => {
  const { theme } = useTheme();
  const publicClient = usePublicClient();
  const queryClient = useQueryClient();
  const { subscriptions } = useApp();
  const { coins: allCoins } = useConfig();
  const widgetRef = useRef<TV.IChartingLibraryWidget | null>(null);

  const dataFeed = useMemo(
    () =>
      createTradingViewDataFeed({
        client: publicClient,
        queryClient,
        subscriptions,
        coins: allCoins.bySymbol,
      }),
    [allCoins, queryClient, publicClient],
  );

  useEffect(() => {
    const toolbar_bg = theme === "dark" ? "#363432" : "#FFF9F0";
    const widget = new TV.widget({
      container: "tv_chart_container",
      autosize: true,
      symbol: `${coins.base.symbol}-${coins.quote.symbol}`,
      interval: "5" as TV.ResolutionString,
      locale: "en",
      library_path: "/static/charting_library/",
      custom_css_url: "/styles/tv-overrides.css",
      theme,
      datafeed: dataFeed,
      loading_screen: {
        backgroundColor: "transparent",
        foregroundColor: "rgb(249 169 178)",
      },
      disabled_features: ["header_symbol_search", "header_compare"],
      overrides: {
        "mainSeriesProperties.candleStyle.upColor": "#27AE60",
        "mainSeriesProperties.candleStyle.downColor": "#EB5757",
        "mainSeriesProperties.candleStyle.borderUpColor": "#27AE60",
        "mainSeriesProperties.candleStyle.borderDownColor": "#EB5757",
        "mainSeriesProperties.candleStyle.wickUpColor": "#27AE60",
        "mainSeriesProperties.candleStyle.wickDownColor": "#EB5757",

        "paneProperties.backgroundType": "solid",
        "paneProperties.background": toolbar_bg,
      },

      studies_overrides: {
        "volume.volume.color.0": "#EB5757",
        "volume.volume.color.1": "#27AE60",
        "volume.volume.transparency": 50,
      },
    });

    widget.onChartReady(async () => {
      const chart = widget.activeChart();
      widgetRef.current = widget;
      widget.applyOverrides({ "paneProperties.background": toolbar_bg });

      /*  orders.forEach((order) => {
        chart.createShape({
          time: order.timestamp,
          price: order.price,
          text: `${order.type} ${order.amount} ${coins.base.symbol}`,
          color: order.type === "buy" ? "#27AE60" : "#EB5757",
          shape: "label",
        });
      }); */
    });
  }, []);

  useEffect(() => {
    if (!widgetRef.current) return;
    widgetRef.current.setSymbol(
      `${coins.base.symbol}-${coins.quote.symbol}`,
      "5" as TV.ResolutionString,
      () => {},
    );
  }, [coins]);

  return <div id="tv_chart_container" className="w-full lg:min-h-[52vh] h-full" />;
};
