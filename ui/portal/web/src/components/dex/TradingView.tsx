import type React from "react";
import { useEffect, useRef } from "react";
import * as TV from "@left-curve/tradingview";
import { UDFCompatibleDatafeed } from "@left-curve/tradingview/datafeeds/udf";

import type { AnyCoin } from "@left-curve/store/types";
import type { OrdersByUserResponse } from "@left-curve/dango/types";
import { useTheme } from "@left-curve/applets-kit";

type TradingViewProps = {
  coins: { base: AnyCoin; quote: AnyCoin };
  orders: OrdersByUserResponse[];
};

export const TradingView: React.FC<TradingViewProps> = ({ coins, orders }) => {
  const { theme } = useTheme();
  const chartContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const toolbar_bg = theme === "dark" ? "#2D2C2A" : "#FFFCF6";
    new TV.widget({
      container: "tv_chart_container",
      autosize: true,
      symbol: "AAPL",
      interval: "1D" as any,
      locale: "en",
      theme: "dark",
      library_path: "/static/charting_library/",
      datafeed: new UDFCompatibleDatafeed("https://demo-feed-data.tradingview.com", undefined, {
        maxResponseLength: 1000,
        expectedOrder: "latestFirst",
      }),
      enabled_features: ["header_screenshot"],
      overrides: {
        "mainSeriesProperties.candleStyle.upColor": "#27AE60",
        "mainSeriesProperties.candleStyle.downColor": "#EB5757",
        "mainSeriesProperties.candleStyle.borderUpColor": "#27AE60",
        "mainSeriesProperties.candleStyle.borderDownColor": "#EB5757",
        "mainSeriesProperties.candleStyle.wickUpColor": "#27AE60",
        "mainSeriesProperties.candleStyle.wickDownColor": "#EB5757",

        "mainSeriesProperties.areaStyle.color1": "#606090",
      },

      studies_overrides: {
        "volume.volume.color.0": "#EB5757",
        "volume.volume.color.1": "#27AE60",
        "volume.volume.transparency": 50,
      },
    });
  }, []);

  return <div id="tv_chart_container" className="w-full lg:min-h-[52vh] h-full" />;
};
