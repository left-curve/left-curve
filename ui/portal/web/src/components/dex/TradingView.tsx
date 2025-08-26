import type React from "react";
import { useEffect, useRef } from "react";
import { useApp } from "~/hooks/useApp";
import { useTheme } from "@left-curve/applets-kit";
import { useConfig, usePublicClient, useStorage } from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";

import * as TV from "@left-curve/tradingview";
import { createTradingViewDataFeed } from "~/datafeed";
import { Direction } from "@left-curve/dango/types";

import type { AnyCoin } from "@left-curve/store/types";
import type { OrdersByUserResponse, WithId } from "@left-curve/dango/types";
import { Decimal } from "@left-curve/dango/utils";

type TradingViewProps = {
  coins: { base: AnyCoin; quote: AnyCoin };
  orders: WithId<OrdersByUserResponse>[];
};

export const TradingView: React.FC<TradingViewProps> = ({ coins, orders }) => {
  const pairSymbol = `${coins.base.symbol}-${coins.quote.symbol}`;

  const { theme } = useTheme();
  const publicClient = usePublicClient();
  const queryClient = useQueryClient();
  const { subscriptions } = useApp();
  const { coins: allCoins } = useConfig();
  const { base, quote } = coins;

  const [chartState, setChartState] = useStorage<object>(`tv.${pairSymbol}`, {
    sync: true,
  });

  const [drawnOrders, setDrawnOrders] = useStorage<Map<string, TV.EntityId>>("tv.drawnOrders", {
    sync: true,
    initialValue: new Map(),
  });

  const widgetRef = useRef<TV.IChartingLibraryWidget | null>(null);

  useEffect(() => {
    const toolbar_bg = theme === "dark" ? "#363432" : "#FFF9F0";
    const toTimestamp = Math.floor(Date.now() / 1000);
    const fromTimestamp = toTimestamp - 3600 * 4;

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
      timeframe: {
        from: fromTimestamp,
        to: toTimestamp,
      },
      loading_screen: {
        backgroundColor: "transparent",
        foregroundColor: "rgb(249 169 178)",
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
      },

      studies_overrides: {
        "volume.volume.color.0": "#EB5757",
        "volume.volume.color.1": "#27AE60",
        "volume.volume.transparency": 50,
      },
    });

    const saveFn = () => widget.save(setChartState);

    widget.onChartReady(() => {
      widget.subscribe("onAutoSaveNeeded", saveFn);
      widget.applyOverrides({ "paneProperties.background": toolbar_bg });
      widgetRef.current = widget;
    });

    return () => {
      widgetRef.current?.remove();
      widgetRef.current = null;
    };
  }, []);

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
    const ordersId = new Set(orders.map((o) => o.id));

    for (const [orderId, shapeId] of drawnOrders) {
      if (ordersId.has(orderId)) continue;
      chart.removeEntity(shapeId);
      drawnOrders.delete(orderId);
    }

    for (const order of orders) {
      if (drawnOrders.has(order.id)) continue;

      /*   chart.createOrderLine().then((l) => {
        const price = Decimal(order.price)
          .times(Decimal(10).pow(base.decimals - quote.decimals))
          .toFixed(5);

        const orderLine = l
          .setPrice(+price)
          .setLineStyle(2)
          .setLineColor(order.direction === Direction.Buy ? "#27AE60" : "#EB5757")
          .setQuantityBackgroundColor(order.direction === Direction.Buy ? "#27AE60" : "#EB5757")
        drawnOrders.set(order.id, orderLine);
      }); */

      const price = Decimal(order.price)
        .times(Decimal(10).pow(base.decimals - quote.decimals))
        .toFixed(5);

      const color = order.direction === Direction.Buy ? "#27AE60" : "#EB5757";

      chart
        .createShape(
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
        )
        .then((id) => drawnOrders.set(order.id, id));
    }
    setDrawnOrders(new Map(drawnOrders));
  }, [orders]);

  return <div id="tv-container" className="w-full lg:min-h-[52vh] h-full" />;
};
