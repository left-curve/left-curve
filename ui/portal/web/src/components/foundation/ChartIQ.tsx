// @ts-nocheck

import "@left-curve/chartiq/js/standard";
import "@left-curve/chartiq/js/advanced";
import "@left-curve/chartiq/js/componentUI";
import "@left-curve/chartiq/js/addOns";

import { CIQ } from "@left-curve/chartiq/js/components";

import { useMediaQuery, useTheme } from "@left-curve/applets-kit";
import { useConfig, usePublicClient } from "@left-curve/store";
import { useEffect, useRef, useState } from "react";
import { useApp } from "~/hooks/useApp";

import { createChartIQConfig, createChartIQDataFeed } from "~/chartiq";

import "@left-curve/chartiq/examples/translations/translationSample";

import "@left-curve/chartiq/css/normalize.css";
import "@left-curve/chartiq/css/stx-chart.css";
import "@left-curve/chartiq/css/chartiq.css";
import "@left-curve/chartiq/css/webcomponents.css";

import type React from "react";

export const ChartIQ = ({ coins }) => {
  const [context, setContext] = useState<CIQ.UI.Context | null>(null);
  const container = useRef<HTMLElement | null>(null);
  const isMounted = useRef(false);
  const { isMd } = useMediaQuery();

  const publicClient = usePublicClient();
  const { subscriptions } = useApp();
  const { coins: allCoins } = useConfig();

  const { current: dataFeed } = useRef(
    createChartIQDataFeed({
      client: publicClient,
      subscriptions,
      updateChartData: (params) => context?.stx?.updateChartData(params),
      coins: allCoins,
    }),
  );

  const { theme } = useTheme();

  const { base, quote } = coins;

  const pairSymbol = `${base.symbol}-${quote.symbol}`;

  useEffect(() => {
    if (!isMounted.current) {
      const uiContext = new CIQ.UI.Chart().createChartAndUI({
        container: container.current!,
        config: createChartIQConfig({
          pairSymbol,
          dataFeed,
          theme,
        }),
      });

      setContext(uiContext);

      const { stx } = uiContext;

      dataFeed.setStx(stx);

      if (isMd) {
        const { channelWrite } = CIQ.UI.BaseComponent.prototype;
        channelWrite(stx.uiContext.config.channels.drawing, true, stx);
      }

      const volumeColor =
        theme === "dark" ? { up: "#66C86A", down: "#FF6B6B" } : { up: "#25B12A", down: "#E71818" };

      CIQ.Studies.addStudy(
        stx,
        "volume",
        { id: "Volume" },
        { "Up Volume": volumeColor.up, "Down Volume": volumeColor.down },
      );

      stx.setPeriodicity(5, "minute", 0, true);

      stx.candleWidthPercent = 0.9;
      stx.chart.yAxis.zoom = -0.0000001;
      stx.controls.mSticky = false;

      stx.animations.zoom = new CIQ.EaseMachine("easeOutCubic", 1);
      stx.swipeRelease = () => {};

      stx.controls.chartControls.style.display = "none";
      stx.controls.chartControls = null;
      stx.layout.smartzoom = false;
      stx.highlightPrimarySeries = false;
      Object.assign(window, { stx, CIQ });

      isMounted.current = true;
    }

    return () => {
      if (context) {
        context.stx.destroy();
        context.stx.draw = () => {};
        setContext(null);
      }
    };
  }, []);

  useEffect(() => {
    if (!isMounted.current || !context) return;
    context.stx.chartId = pairSymbol;
    context.stx.chart.symbol = pairSymbol;
    context.changeSymbol({ symbol: pairSymbol });
  }, [pairSymbol]);

  return (
    <div className="w-full min-h-[23.1375rem] lg:min-h-[52vh] h-full relative">
      <cq-context ref={container} className="chart-context">
        <cq-chart-instructions />

        <nav className="ciq-nav full-screen-hide">
          {!isMd ? (
            <>
              <cq-toggle
                class="ciq-draw"
                member="drawing"
                reader="Draw"
                tooltip="Draw"
                icon="draw"
                help-id="drawing_tools_toggle"
              />
            </>
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
          <div className="ciq-menu-section">
            <div className="ciq-dropdowns">
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
                <cq-drawing-settings
                  class="palette-settings"
                  docked="true"
                  hide="true"
                  orientation="horizontal"
                  min-height="40"
                  cq-drawing-edit="none"
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
