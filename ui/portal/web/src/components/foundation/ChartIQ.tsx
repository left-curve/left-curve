// @ts-nocheck

import "@left-curve/chartiq/js/standard";
import "@left-curve/chartiq/js/advanced";
import "@left-curve/chartiq/js/componentUI";
import "@left-curve/chartiq/js/addOns";

import { CIQ } from "@left-curve/chartiq/js/components";
import getDefaultConfig from "@left-curve/chartiq/js/defaultConfiguration";

import getLicenseKey from "@left-curve/chartiq/license/key";

getLicenseKey(CIQ);

import { useEffect, useRef, useState } from "react";

import "@left-curve/chartiq/examples/translations/translationSample";
import quotefeed from "@left-curve/chartiq/examples/feeds/quoteFeedSimulator.js";

import "@left-curve/chartiq/css/normalize.css";
import "@left-curve/chartiq/css/stx-chart.css";
import "@left-curve/chartiq/css/chartiq.css";
import "@left-curve/chartiq/css/webcomponents.css";

import type React from "react";

declare global {
  namespace JSX {
    interface IntrinsicElements {
      "cq-chart-instructions": React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
      "cq-context": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-side-panel": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-dialogs": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-dialog": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-drawing-context": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-abstract-marker": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-attribution": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-loader": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-palette-dock": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-drawing-palette": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-drawing-settings": React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
      "cq-toggle": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-menu": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-side-nav": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-chart-title": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
      "cq-marker": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
    }
  }
}

import sample5min from "@left-curve/chartiq/examples/data/STX_SAMPLE_5MIN";

export const ChartIQ = () => {
  const [stx, setStx] = useState<CIQ.ChartEngine | null>(null);
  const container = useRef<HTMLElement | null>(null);
  const loading = useRef(true);

  useEffect(() => {
    if (loading.current) {
      const config = getDefaultConfig({
        quoteFeed: quotefeed,
      });

      const {
        onNewSymbolLoad,
        hotkeyConfig,
        lookupDriver,
        systemMessages,
        marketFactory,
        quoteFeeds,
        selector,
        themes,
        menuChartStyle,
        menuChartAggregates,
        menuChartPreferences,
        menuYAxisPreferences,
        menuStudiesConfig,
        menuAddOns,
        menuRendering,
        getMenu,
        eventMarkersImplementation,
        scrollbarStyling,
        nameValueStore,
        createChart,
        onWebComponentsReady,
        onChartReady,
        onEngineReady,
        onMultiChartEvent,
        soloActive,
        useQueryString,
        updateFromQueryString,
        chartEngineParams,
        root,
        plugins,
        menus,
        ...rest
      } = config;

      console.log(config);
      const customConfig = {
        ...rest,
        chartId: "BTC-USDC",
        menus: {
          ...menus,
          events: {
            content: [],
          },
          markers: {
            content: [
              {
                type: "heading",
                label: "SignalIQ",
                feature: "signaliq",
                menuPersist: true,
              },
              {
                type: "heading",
                label: "Chart Events",
                menuPersist: true,
              },
              {
                type: "switch",
                label: "Orders",
                setget: "Markers.MarkerSwitch",
                value: "square",
              },
            ],
          },
          period: {
            content: [
              {
                type: "item",
                label: "1 D",
                tap: "Layout.setPeriodicity",
                value: [1, 1, "day"],
              },
              {
                type: "item",
                label: "1 W",
                tap: "Layout.setPeriodicity",
                value: [1, 1, "week"],
              },
              {
                type: "separator",
                menuPersist: true,
              },
              {
                type: "item",
                label: "1 Min",
                tap: "Layout.setPeriodicity",
                value: [1, 1, "minute"],
              },
              {
                type: "item",
                label: "5 Min",
                tap: "Layout.setPeriodicity",
                value: [1, 5, "minute"],
              },
              {
                type: "item",
                label: "15 Min",
                tap: "Layout.setPeriodicity",
                value: [3, 5, "minute"],
              },
              {
                type: "item",
                label: "1 Hour",
                tap: "Layout.setPeriodicity",
                value: [2, 30, "minute"],
              },
              {
                type: "item",
                label: "4 Hour",
                tap: "Layout.setPeriodicity",
                value: [8, 30, "minute"],
              },
              {
                type: "separator",
                menuPersist: true,
              },
              {
                type: "item",
                label: "1 Sec",
                tap: "Layout.setPeriodicity",
                value: [1, 1, "second"],
              },
            ],
          },
          preferences: {
            content: [
              {
                type: "heading",
                label: "Chart Preferences",
                menuPersist: true,
              },
              {
                type: "switch",
                label: "Range Selector",
                setget: "Layout.RangeSlider",
                feature: "rangeslider",
                menuPersist: true,
              },
              {
                type: "switch",
                label: "Animation",
                setget: "Layout.Animation",
                feature: "animation",
                menuPersist: true,
              },
              {
                type: "switch",
                label: "Hide Outliers",
                setget: "Layout.Outliers",
                feature: "outliers",
                menuPersist: true,
              },
              {
                type: "switch",
                label: "Market Depth",
                setget: "Layout.MarketDepth",
                feature: "marketdepth",
                menuPersist: true,
              },
              {
                type: "switch",
                label: "L2 Heat Map",
                setget: "Layout.L2Heatmap",
                feature: "marketdepth",
                menuPersist: true,
              },
              {
                type: "separator",
                menuPersist: true,
              },
              {
                type: "heading",
                label: "Y-Axis Preferences",
                menuPersist: true,
              },
              {
                type: "switch",
                label: "Log Scale",
                setget: "Layout.ChartScale",
                value: "log",
                menuPersist: true,
              },
              {
                type: "switch",
                label: "Invert",
                setget: "Layout.FlippedChart",
                menuPersist: true,
              },
              {
                type: "separator",
                menuPersist: true,
              },
              {
                type: "heading",
                label: "Chart Preferences",
                menuPersist: true,
              },
              {
                type: "radio",
                label: "Hide Heads-Up Display",
                setget: "Layout.HeadsUp",
                value: "crosshair",
              },
              {
                type: "radio",
                label: "Show Heads-Up Display",
                setget: "Layout.HeadsUp",
                value: "static",
              },
              {
                type: "separator",
                menuPersist: true,
              },
              /*        {
                type: "item",
                label: "Shortcuts / Hotkeys",
                tap: "Layout.showShortcuts",
                value: true,
                feature: "shortcuts",
              }, */
              {
                type: "heading",
                label: "Locale",
                menuPersist: true,
              },
              {
                type: "clickable",
                label: "Change Timezone",
                selector: "cq-timezone-dialog",
                method: "open",
              },
              {
                type: "item",
                label: "Change Language",
                setget: "Layout.Language",
                iconCls: "flag",
              },
            ],
          },
        },
        root,
        initialSymbol: {
          symbol: "BTC-USDC",
          name: "Bitcoin",
          exchDisp: "Dango",
        },
        initialData: sample5min,
        enabledAddOns: {
          tooltip: {
            ohl: true,
            volume: true,
            series: true,
            studies: true,
            enabled: true,
          },

          inactivityTimer: {
            minutes: 30,
            enabled: true,
          },

          animation: {
            animationParameters: { tension: 0.3 },
            enabled: false,
          },

          outliers: { enabled: false },

          rangeSlider: { enabled: true },

          fullScreen: { enabled: true },

          extendedHours: {
            filter: true,
            enabled: true,
          },

          continuousZoom: {
            periodicities: [
              // Daily interval data
              { period: 1, interval: "month" },
              { period: 1, interval: "week" },
              { period: 1, interval: "day" },
              // 30 minute interval data
              { period: 8, interval: 30 },
              { period: 1, interval: 30 },
              // 1 minute interval data
              { period: 5, interval: 1 },
              { period: 1, interval: 1 },
              // One second interval data
              { period: 10, interval: 1, timeUnit: "second" },
              { period: 1, interval: 1, timeUnit: "second" },
            ],
            boundaries: {
              maxCandleWidth: 15,
              minCandleWidth: 3,
            },
            enabled: false,
          },

          forecasting: { enabled: false },

          tableView: { enabled: true },

          dataLoader: { enabled: true },

          shortcuts: { enabled: true },
        },
        onNewSymbolLoad,
        restore: true,
        lookupDriver,
        hotkeyConfig,
        systemMessages,
        marketFactory,
        chartEngineParams: {
          preferences: {
            currentPriceLine: true,
            currentPriceLabel: true,
            whitespace: 0,
          },
          chart: {
            yAxis: {
              position: "right",
            },
          },
        },
        quoteFeeds,
        selector,
        themes: {
          builtInThemes: {
            "ciq-day": "Day",
            "ciq-day.redgreen": "Day (red-green friendly)",
            "ciq-night": "Night",
            "ciq-night.redgreen": "Night (red-green friendly)",
          },
          defaultTheme: "ciq-day",
        },
        menuPeriodicity: [
          {
            type: "item",
            label: "1 D",
            cmd: "Layout.setPeriodicity(1,1,'day')",
            value: { period: 1, interval: 1, timeUnit: "day" },
          },
          {
            type: "item",
            label: "1 W",
            cmd: "Layout.setPeriodicity(1,1,'week')",
            value: { period: 1, interval: 1, timeUnit: "week" },
          },
          {
            type: "item",
            label: "1 Mo",
            cmd: "Layout.setPeriodicity(1,1,'month')",
            value: { period: 1, interval: 1, timeUnit: "month" },
          },
          { type: "separator" },
          {
            type: "item",
            label: "1 Min",
            cmd: "Layout.setPeriodicity(1,1,'minute')",
            value: { period: 1, interval: 1, timeUnit: "minute" },
          },
          {
            type: "item",
            label: "5 Min",
            cmd: "Layout.setPeriodicity(1,5,'minute')",
            value: { period: 1, interval: 5, timeUnit: "minute" },
          },
          {
            type: "item",
            label: "10 Min",
            cmd: "Layout.setPeriodicity(1,10,'minute')",
            value: { period: 1, interval: 10, timeUnit: "minute" },
          },
          {
            type: "item",
            label: "15 Min",
            cmd: "Layout.setPeriodicity(3,5,'minute')",
            value: { period: 1, interval: 1, timeUnit: "minute" },
          },
          {
            type: "item",
            label: "30 Min",
            cmd: "Layout.setPeriodicity(1,30,'minute')",
            value: { period: 1, interval: 30, timeUnit: "minute" },
          },
          {
            type: "item",
            label: "1 Hour",
            cmd: "Layout.setPeriodicity(2,30,'minute')",
            value: { period: 2, interval: 30, timeUnit: "minute" },
          },
          {
            type: "item",
            label: "4 Hour",
            cmd: "Layout.setPeriodicity(8,30,'minute')",
            value: { period: 8, interval: 30, timeUnit: "minute" },
          },
          { type: "separator" },
          {
            type: "item",
            label: "1 Sec",
            cmd: "Layout.setPeriodicity(1,1,'second')",
            value: { period: 1, interval: 1, timeUnit: "second" },
          },
          {
            type: "item",
            label: "10 Sec",
            cmd: "Layout.setPeriodicity(1,10,'second')",
            value: { period: 1, interval: 10, timeUnit: "second" },
          },
          {
            type: "item",
            label: "30 Sec",
            cmd: "Layout.setPeriodicity(1,30,'second')",
            value: { period: 1, interval: 30, timeUnit: "second" },
          },
        ],
        menuChartStyle,
        menuChartAggregates,
        menuChartPreferences: [
          {
            type: "checkbox",
            label: "Range Selector",
            cmd: "Layout.RangeSlider()",
            cls: "rangeslider-ui",
          },
          {
            type: "checkbox",
            label: "Extended Hours",
            cmd: "Layout.ExtendedHours()",
            cls: "extendedhours-ui",
          },
          /* Begin Technical Analysis only */
          {
            type: "checkbox",
            label: "Hide Outliers",
            cmd: "Layout.Outliers()",
            cls: "outliers-ui",
          },
          {
            type: "checkbox",
            label: "Market Depth",
            cmd: "Layout.MarketDepth()",
            cls: "marketdepth-ui",
          }, // v8.2.0 cls changed from cryptoiq-ui
          {
            type: "checkbox",
            label: "L2 Heat Map",
            cmd: "Layout.L2Heatmap()",
            cls: "marketdepth-ui",
          }, // v8.2.0 cls changed from cryptoiq-ui
          /* End Technical Analysis only */
        ],
        menuYAxisPreferences,
        menuViewConfig: {},
        menuStudiesConfig,
        menuAddOns,
        rangeMenu: [
          { type: "range", label: "1D", cmd: "set(1,'today')" },
          { type: "range", label: "5D", cmd: "set(5,'day',30,2,'minute')" },
          { type: "range", label: "1M", cmd: "set(1,'month',30,8,'minute')" },
          { type: "range", label: "3M", cmd: "set(3,'month')", cls: "hide-sm" },
          { type: "range", label: "6M", cmd: "set(6,'month')", cls: "hide-sm" },
          { type: "range", label: "YTD", cmd: "set(1,'YTD')", cls: "hide-sm" },
          { type: "range", label: "1Y", cmd: "set(1,'year')" },
          { type: "range", label: "5Y", cmd: "set(5,'year',1,1,'week')", cls: "hide-sm" },
          { type: "range", label: "All", cmd: "set(1,'all')", cls: "hide-sm" },
        ],
        drawingTools: [
          { type: "dt", tool: "annotation", group: "text", label: "Annotation", shortcut: "t" },
          { type: "dt", tool: "arrow", group: "markings", label: "Arrow", shortcut: "a" },
          { type: "dt", tool: "line", group: "lines", label: "Line", shortcut: "l" },
          { type: "dt", tool: "horizontal", group: "lines", label: "Horizontal", shortcut: "h" },
          { type: "dt", tool: "vertical", group: "lines", label: "Vertical", shortcut: "v" },
          { type: "dt", tool: "rectangle", group: "markings", label: "Rectangle", shortcut: "r" },
          { type: "dt", tool: "segment", group: "lines", label: "Segment" },
          /* Begin Technical Analysis only */
          { type: "dt", tool: "callout", group: "text", label: "Callout" },
          { type: "dt", tool: "average", group: "statistics", label: "Average Line" },
          { type: "dt", tool: "channel", group: "lines", label: "Channel" },
          { type: "dt", tool: "continuous", group: "lines", label: "Continuous" },
          { type: "dt", tool: "crossline", group: "lines", label: "Crossline" },
          { type: "dt", tool: "freeform", group: "lines", label: "Doodle" },
          { type: "dt", tool: "elliottwave", group: "technicals", label: "Elliott Wave" },
          { type: "dt", tool: "ellipse", group: "markings", label: "Ellipse", shortcut: "e" },
          { type: "dt", tool: "retracement", group: "fibonacci", label: "Fib Retracement" },
          { type: "dt", tool: "fibprojection", group: "fibonacci", label: "Fib Projection" },
          { type: "dt", tool: "fibarc", group: "fibonacci", label: "Fib Arc" },
          { type: "dt", tool: "fibfan", group: "fibonacci", label: "Fib Fan" },
          { type: "dt", tool: "fibtimezone", group: "fibonacci", label: "Fib Time Zone" },
          { type: "dt", tool: "gannfan", group: "technicals", label: "Gann Fan" },
          { type: "dt", tool: "gartley", group: "technicals", label: "Gartley" },
          { type: "dt", tool: "pitchfork", group: "technicals", label: "Pitchfork" },
          { type: "dt", tool: "quadrant", group: "statistics", label: "Quadrant Lines" },
          { type: "dt", tool: "ray", group: "lines", label: "Ray" },
          { type: "dt", tool: "regression", group: "statistics", label: "Regression Line" },
          { type: "dt", tool: "check", group: "markings", label: "Check" },
          { type: "dt", tool: "xcross", group: "markings", label: "Cross" },
          { type: "dt", tool: "focusarrow", group: "markings", label: "Focus" },
          { type: "dt", tool: "heart", group: "markings", label: "Heart" },
          { type: "dt", tool: "star", group: "markings", label: "Star" },
          { type: "dt", tool: "speedarc", group: "technicals", label: "Speed Resistance Arc" },
          { type: "dt", tool: "speedline", group: "technicals", label: "Speed Resistance Line" },
          { type: "dt", tool: "timecycle", group: "technicals", label: "Time Cycle" },
          { type: "dt", tool: "tirone", group: "statistics", label: "Tirone Levels" },
          { type: "dt", tool: "trendline", group: "text", label: "Trend Line" },
          /* End Technical Analysis only */
        ],
        drawingToolGrouping: [
          "All",
          "Favorites",
          "Text",
          /* Begin Technical Analysis only */
          "Statistics",
          "Technicals",
          "Fibonacci",
          /* End Technical Analysis only */
          "Markings",
          "Lines",
        ],
        menuRendering,
        getMenu,
        plugins: {
          marketDepth: {
            volume: true,
            mountain: true,
            step: true,
            record: true,
            height: "50%",
            orderbook: true,
            allowUIZoom: true, // v8.2.1
          },
        },
        channels: {
          crosshair: "layout.crosshair",
          headsUp: "layout.headsUp",
          sidenav: "layout.sidenav",
          tableView: "channel.tableView", // v8.1.0
          drawing: "channel.drawing",
          drawingPalettes: "channel.drawingPalettes",
          breakpoint: "channel.breakpoint",
          containerSize: "channel.containerSize",
          sidenavSize: "channel.sidenavSize",
          sidepanelSize: "channel.sidepanelSize",
          pluginPanelHeight: "channel.pluginPanelHeight",
          tfc: "channel.tfc",
          tc: "channel.tc",
          technicalviews: "channel.technicalviews", // v8.1.0
          technicalinsights: "channel.technicalinsights", // v8.1.0 changed from recognia: "channel.recognia"
          dialog: "channel.dialog",
          keyboardNavigation: "channel.keyboardNavigation", // v8.2.0
        },
        dialogs: {
          view: { tag: "cq-view-dialog" },
          aggregation: { tag: "cq-aggregation-dialog" },
          timezone: { tag: "cq-timezone-dialog" },
          language: { tag: "cq-language-dialog" },
          theme: { tag: "cq-theme-dialog" },
          study: {
            tag: "cq-study-dialog",
            attributes: {
              "cq-study-axis": true,
              "cq-study-panel": "alias",
            },
          },
          fibSettings: { tag: "cq-fib-settings-dialog" },
          share: { tag: "cq-share-dialog" },
        },
        eventMarkersImplementation,
        scrollbarStyling,
        multiChartCopySymbol: null,
        multiChartLoadMsg: "",
        nameValueStore,
        onWebComponentsReady,
        onChartReady,
        onEngineReady,
        onMultiChartEvent,
        createChart,
        soloActive,
        useQueryString,
        updateFromQueryString,
      };

      const { stx } = new CIQ.UI.Chart().createChartAndUI({
        container: container.current!,
        config: customConfig,
      });

      stx.chart.yAxis.zoom = -0.0000001;
      stx.controls.mSticky = false;

      stx.animations.zoom = new CIQ.EaseMachine("easeOutCubic", 1);
      stx.swipeRelease = () => {};

      stx.controls.chartControls.style.display = "none";
      stx.controls.chartControls = null;
      stx.layout.smartzoom = false;

      setStx(stx);
      Object.assign(window, { stx, CIQ });
      loading.current = false;
    }

    return () => {
      if (stx) {
        stx.destroy();
        stx.draw = () => {};
        setStx(null);
      }
    };
  }, []);

  return (
    <div className="w-full lg:min-h-[52vh] h-full relative">
      <cq-context ref={container} className="chart-context">
        <cq-chart-instructions />

        <nav className="ciq-nav full-screen-hide">
          <div className="sidenav-toggle ciq-toggles">
            <cq-toggle
              class="ciq-sidenav"
              member="sidenav"
              toggles="sidenavOn,sidenavOff"
              toggle-classes="active,"
              reader="More Options"
              tooltip="More"
              icon="morenav"
            />
          </div>

          <cq-side-nav cq-on="sidenavOn">
            <cq-toggle
              class="ciq-draw"
              member="drawing"
              reader="Draw"
              tooltip="Draw"
              icon="draw"
              help-id="drawing_tools_toggle"
            />
            {/*      <cq-toggle
              class="ciq-CH"
              config="crosshair"
              reader="Crosshair"
              tooltip="Crosshair (Alt + \)"
              icon="crosshair"
            />
            <cq-menu
              class="nav-dropdown toggle-options"
              reader="Crosshair Options"
              config="crosshair"
            /> */}
            {/*    <cq-toggle
              class="ciq-HU"
              feature="tooltip"
              config="info"
              reader="Info"
              tooltip="Info"
              icon="info"
            />
            <cq-menu
              feature="tooltip"
              class="nav-dropdown toggle-options"
              reader="Info Options"
              config="info"
            /> */}

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
              class="nav-dropdown ciq-markers alignright"
              config="markers"
              text="Events"
              icon="events"
              responsive=""
              tooltip="Events"
            />
          </cq-side-nav>

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
                  class="palette-drawing palette-hide !flex pb-2"
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
              <table className="hu-tooltip">
                <caption>Tooltip</caption>
                <tbody>
                  <tr hu-tooltip-field="" className="hu-tooltip-sr-only">
                    <th>Field</th>
                    <th>Value</th>
                  </tr>
                  <tr hu-tooltip-field="DT">
                    <td className="hu-tooltip-name">Date/Time</td>
                    <td className="hu-tooltip-value" />
                  </tr>
                  <tr hu-tooltip-field="Close">
                    <td className="hu-tooltip-name" />
                    <td className="hu-tooltip-value" />
                  </tr>
                </tbody>
              </table>

              <cq-chart-title
                cq-marker=""
                cq-browser-tab=""
                cq-activate-symbol-search-on-click=""
              />

              <cq-marker class="chart-control-group full-screen-show">
                <cq-toggle
                  class="ciq-lookup-icon"
                  config="symbolsearch"
                  reader="Symbol Search"
                  tooltip="Symbol Search"
                  icon="search"
                  help-id="search_symbol_lookup"
                />
                <cq-toggle
                  class="ciq-comparison-icon"
                  config="symbolsearch"
                  reader="Add Comparison"
                  tooltip="Add Comparison"
                  icon="compare"
                  help-id="add_comparison"
                  comparison="true"
                />
                <cq-toggle
                  class="ciq-draw"
                  member="drawing"
                  reader="Draw"
                  icon="draw"
                  tooltip="Draw"
                  help-id="drawing_tools_toggle"
                />
                <cq-toggle
                  class="ciq-CH"
                  config="crosshair"
                  reader="Crosshair"
                  icon="crosshair"
                  tooltip="Crosshair (Alt + \)"
                />
                <cq-toggle
                  class="ciq-DT"
                  feature="tableview"
                  member="tableView"
                  reader="Table View"
                  icon="tableview"
                  tooltip="Table View"
                />
                <cq-menu
                  class="nav-dropdown ciq-period full-screen"
                  config="period"
                  text=""
                  binding="Layout.periodicity"
                />
              </cq-marker>

              <cq-loader />
            </div>
          </div>
        </div>

        <cq-abstract-marker cq-type="helicopter" />

        <cq-attribution />
        {/*       <div role="complementary" className="ciq-footer full-screen-hide">
        <cq-share-button
          class="ciq-share-button bottom"
          reader="Share Chart"
          icon="share"
          tooltip="Share"
        ></cq-share-button>
        <cq-toggle
          feature="shortcuts"
          class="ciq-shortcut-button bottom"
          stxtap="Layout.showShortcuts()"
          reader="Toggle Shortcut Legend"
          icon="shortcuts"
          tooltip="Shortcuts"
        ></cq-toggle>
        <cq-toggle
          feature="help"
          class="ciq-help-button bottom"
          stxtap="Layout.toggleHelp()"
          reader="Toggle Interactive Help"
          icon="help"
          tooltip="Interactive Help"
        ></cq-toggle>
        <cq-show-range
          config="range"
          role="group"
          aria-labelledby="label_showRange"
        ></cq-show-range>
      </div> */}

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
