// @ts-nocheck

import "@left-curve/chartiq/js/standard";
import "@left-curve/chartiq/js/componentUI";
import "@left-curve/chartiq/js/addOns";

import { useMediaQuery, useTheme, useStorage } from "@left-curve/applets-kit";
import { CIQ } from "@left-curve/chartiq/js/components";
import { useConfig, usePublicClient } from "@left-curve/store";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { createChartIQConfig, createChartIQDataFeed } from "~/chartiq";
import { useApp } from "~/hooks/useApp";
import { useQueryClient } from "@tanstack/react-query";

import { Decimal, formatNumber } from "@left-curve/dango/utils";

import "@left-curve/chartiq/examples/translations/translationSample";

import "@left-curve/chartiq/css/normalize.css";
import "@left-curve/chartiq/css/stx-chart.css";
import "@left-curve/chartiq/css/chartiq.css";
import "@left-curve/chartiq/css/webcomponents.css";

import type { AnyCoin } from "@left-curve/store/types";
import type { OrdersByUserResponse } from "@left-curve/dango/types";

type ChartIQProps = {
  coins: { base: AnyCoin; quote: AnyCoin };
  orders: OrdersByUserResponse[];
};

export const ChartIQ: React.FC<ChartIQProps> = ({ coins, orders }) => {
  const uiContextRef = useRef<CIQ.UI.Context | null>(null);
  const container = useRef<HTMLElement | null>(null);
  const { isMd } = useMediaQuery();
  const { settings } = useApp();

  const { formatNumberOptions } = settings;
  const { base, quote } = coins;

  const pairSymbol = `${base.symbol}-${quote.symbol}`;

  const [chartPreferences, setChartPreferences] = useStorage(`chartiq.${pairSymbol}`, {
    initialValue: {},
    sync: true,
  });

  const changeChartPreferences = useCallback(
    (s: Record<string, unknown>) => setChartPreferences((prev) => ({ ...prev, ...s })),
    [setChartPreferences],
  );

  const handleLayoutChange = useCallback(
    ({ stx }) => changeChartPreferences({ layout: stx.exportLayout(true) }),
    [setChartPreferences],
  );
  const handleDrawingChange = useCallback(
    ({ stx }) => changeChartPreferences({ drawings: stx.exportDrawings() }),
    [setChartPreferences],
  );
  const handlePreferencesChange = useCallback(
    ({ stx }) => changeChartPreferences({ preferences: stx.exportPreferences() }),
    [setChartPreferences],
  );

  const publicClient = usePublicClient();
  const queryClient = useQueryClient();
  const { subscriptions } = useApp();
  const { coins: allCoins } = useConfig();

  const { theme } = useTheme();

  const volumeColor =
    theme === "dark" ? { up: "#66C86A", down: "#FF6B6B" } : { up: "#25B12A", down: "#E71818" };

  const dataFeed = useMemo(
    () =>
      createChartIQDataFeed({
        client: publicClient,
        queryClient,
        subscriptions,
        updateChartData: (params) => uiContextRef.current?.stx?.updateChartData(params),
        coins: allCoins.bySymbol,
      }),
    [allCoins, queryClient, publicClient],
  );

  useEffect(() => {
    if (uiContextRef.current) return;
    const uiContext = new CIQ.UI.Chart().createChartAndUI({
      container: container.current!,
      config: createChartIQConfig({
        pairSymbol,
        dataFeed,
        theme,
      }),
    });

    uiContextRef.current = uiContext;

    const { stx } = uiContext;

    dataFeed.setStx(stx);

    if (isMd) {
      const { channelWrite } = CIQ.UI.BaseComponent.prototype;
      channelWrite(stx.uiContext.config.channels.drawing, true, stx);
    }

    CIQ.Studies.addStudy(
      stx,
      "volume",
      { id: "Volume" },
      { "Up Volume": volumeColor.up, "Down Volume": volumeColor.down },
    );

    stx.formatYAxisPrice = (price, params) => {
      if (params?.display?.includes("-")) {
        return formatNumber(price, { ...formatNumberOptions, maxSignificantDigits: 5 });
      }

      return formatNumber(price, { ...formatNumberOptions, notation: "compact" });
    };

    stx.candleWidthPercent = 0.9;
    stx.chart.yAxis.zoom = -0.0000001;
    stx.chart.maxTicks = 40;
    stx.controls.mSticky = false;

    stx.animations.zoom = new CIQ.EaseMachine("easeOutCubic", 1);
    stx.swipeRelease = () => {};

    stx.controls.chartControls.style.display = "none";
    stx.controls.chartControls = null;
    stx.layout.smartzoom = false;
    stx.highlightPrimarySeries = false;

    return () => {
      if (uiContext) {
        uiContext.stx.destroy();
        uiContext.stx.draw = () => {};
        uiContextRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    const ref = uiContextRef.current?.stx.append("draw", () => {
      const stx = uiContextRef.current?.stx;
      for (const order of orders) {
        const baseCoin = allCoins.byDenom[order.baseDenom];
        const quoteCoin = allCoins.byDenom[order.quoteDenom];

        const pairSymbol = `${baseCoin.symbol}-${quoteCoin.symbol}`;
        if (stx.chart.symbol !== pairSymbol) continue;

        const price = Decimal(order.price)
          .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
          .toFixed(5);

        const y = stx.pixelFromPrice(price, stx.chart.panel);
        const color = order.direction === "bid" ? volumeColor.up : volumeColor.down;

        stx.plotLine({
          x0: 0,
          x1: 1,
          y0: y,
          y1: y,
          color,
          type: "horizontal",
          lineWidth: 1,
          context: stx.chart.context,
        });
        stx.createYAxisLabel(
          stx.chart.panel,
          formatNumber(price, { ...formatNumberOptions, maxSignificantDigits: 5 }),
          stx.pixelFromTransformedValue(price),
          color,
          "white",
        );
      }
    });

    return () => {
      uiContextRef.current?.stx.removeInjection(ref);
    };
  }, [orders]);

  useEffect(() => {
    if (!uiContextRef.current) return;
    uiContextRef.current.changeSymbol({ symbol: pairSymbol });

    if (chartPreferences.drawings) {
      uiContextRef.current.stx.importDrawings(chartPreferences.drawings);
    }

    if (chartPreferences.layout) {
      uiContextRef.current.stx.importLayout(chartPreferences.layout, {
        managePeriodicity: true,
        preserveTicksAndCandleWidth: true,
      });
    }

    if (chartPreferences.preferences) {
      uiContextRef.current.stx.importPreferences(chartPreferences.preferences);
    }

    uiContextRef.current.stx.addEventListener("layout", handleLayoutChange);
    uiContextRef.current.stx.addEventListener("drawing", handleDrawingChange);
    uiContextRef.current.stx.addEventListener("preferences", handlePreferencesChange);

    return () => {
      uiContextRef.current?.stx.removeEventListener({ type: "layout", cb: handleLayoutChange });
      uiContextRef.current?.stx.removeEventListener({ type: "drawing", cb: handleDrawingChange });
      uiContextRef.current?.stx.removeEventListener({
        type: "preferences",
        cb: handlePreferencesChange,
      });
      uiContextRef.current?.stx.clearDrawings(true, false);
    };
  }, [pairSymbol]);

  return (
    <div className="w-full min-h-[23.1375rem] lg:min-h-[52vh] h-full relative">
      <cq-context ref={container} className="chart-context">
        <cq-chart-instructions />

        <nav className="ciq-nav full-screen-hide">
          {!isMd ? (
            <cq-toggle
              class="ciq-draw"
              member="drawing"
              reader="Draw"
              tooltip="Draw"
              icon="draw"
              help-id="drawing_tools_toggle"
            />
          ) : null}

          <cq-menu
            class="nav-dropdown ciq-display"
            reader="Display"
            config="display"
            binding="Layout.chartType"
            icon=""
            help-id="display_dropdown"
            tooltip=""
          />
          <cq-menu
            class="nav-dropdown ciq-period"
            reader="Periodicity"
            config="period"
            text=""
            binding="Layout.periodicity"
          />
          <div className="ciq-menu-section">
            <div className="ciq-dropdowns">
              <cq-menu
                class="nav-dropdown ciq-views alignright-md alignright-sm"
                config="views"
                text="Views"
                icon="views"
                responsive=""
                tooltip="Views"
              />
              <cq-menu
                class="nav-dropdown ciq-studies alignright"
                cq-focus="input"
                config="studies"
                text="Studies"
                icon="studies"
                responsive=""
                tooltip="Studies"
              />
              <cq-menu
                class="nav-dropdown ciq-preferences alignright"
                reader="Preferences"
                config="preferences"
                icon="preferences"
                tooltip="Preferences"
              />
            </div>
          </div>
        </nav>

        <div className="ciq-chart-area">
          <div className="ciq-chart">
            <cq-palette-dock>
              <div className="palette-dock-container">
                <cq-drawing-palette
                  class="palette-drawing palette-hide pb-2 !block w-[72px]"
                  docked="true"
                  orientation="vertical"
                  min-height="300"
                  cq-drawing-edit="none"
                  cq-keystroke-claim=""
                />
              </div>
            </cq-palette-dock>

            <div className="chartContainer">
              <cq-chart-title
                cq-marker=""
                cq-browser-tab=""
                cq-activate-symbol-search-on-click=""
              />
              <cq-loader />
            </div>
          </div>
        </div>

        <cq-abstract-marker cq-type="helicopter" />

        <cq-attribution />

        <cq-dialogs>
          <cq-dialog>
            <cq-drawing-context />
          </cq-dialog>
        </cq-dialogs>
        <cq-side-panel />
      </cq-context>
    </div>
  );
};
