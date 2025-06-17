import type React from "react";
import { useEffect, useRef } from "react";

export const TradingViewChart: React.FC = () => {
  const chartContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (chartContainerRef.current) {
      chartContainerRef.current.innerHTML = "";

      const script = document.createElement("script");
      script.src = "https://s3.tradingview.com/tv.js";
      script.async = true;

      script.onload = async () => {
        if (window.TradingView) {
          new window.TradingView.widget({
            container_id: "tv_chart_container",
            autosize: true,
            symbol: "BINANCE:ETHUSDT",
            interval: "15",
            timezone: new Intl.DateTimeFormat().resolvedOptions().timeZone,
            backgroundColor: "rgb(255 249 240)",
            theme: "light",
            enable_publishing: false,
            allow_symbol_change: false,
            withdateranges: true,
            details: false,
            hide_side_toolbar: false,
            save_image: false,
            overrides: {
              "mainSeriesProperties.candleStyle.upColor": "#27AE60",
              "mainSeriesProperties.candleStyle.downColor": "#EB5757",
              "mainSeriesProperties.candleStyle.borderUpColor": "#27AE60",
              "mainSeriesProperties.candleStyle.borderDownColor": "#EB5757",
              "mainSeriesProperties.candleStyle.wickUpColor": "#27AE60",
              "mainSeriesProperties.candleStyle.wickDownColor": "#EB5757",

              "paneProperties.vertGridProperties.color": "#E5E0D8",
              "paneProperties.horzGridProperties.color": "#E5E0D8",

              "mainSeriesProperties.areaStyle.color1": "#606090",

              "paneProperties.crossHairProperties.color": "#000000",
            },

            studies_overrides: {
              "volume.volume.color.0": "#EB5757",
              "volume.volume.color.1": "#27AE60",
              "volume.volume.transparency": 50,
            },
          });
        }
      };

      chartContainerRef.current.appendChild(script);
    }
  }, []);

  return (
    <div
      id="tv_chart_container"
      ref={chartContainerRef}
      className="w-full lg:min-h-[52vh] h-full"
    />
  );
};
