import { createLazyFileRoute } from "@tanstack/react-router";

import { useConfig } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { useHeaderHeight, useMediaQuery } from "@left-curve/applets-kit";

import { ProTrade } from "~/components/dex/components/ProTrade";
import { GridPanel } from "~/components/dex/components/GridPanel";
import { SettingsDrawer } from "~/components/dex/components/SettingsDrawer";
import {
  useGridLayout,
  GRID_COLS,
  GRID_CONTAINER_PADDING,
  GRID_MARGIN,
  GRID_MAX_ROWS,
} from "~/components/dex/hooks/useGridLayout";

import { GridLayout } from "react-grid-layout";
import { useCallback, useEffect, useRef, useState } from "react";

export const Route = createLazyFileRoute("/(app)/_app/trade/$pairSymbols")({
  component: ProTradeApplet,
});

function ProTradeApplet() {
  const navigate = useNavigate();
  const { coins } = useConfig();
  const { pairSymbols } = Route.useParams();
  const { action = "buy", order_type = "market", type = "perps" } = Route.useSearch();
  const headerHeight = useHeaderHeight();
  const { isLg } = useMediaQuery();

  const containerRef = useCallback((node: HTMLDivElement | null) => {
    if (!node) return;
    setContainerWidth(node.offsetWidth);
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) setContainerWidth(entry.contentRect.width);
    });
    observer.observe(node);
    observerRef.current = observer;
  }, []);
  const observerRef = useRef<ResizeObserver | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  const { fullLayout, visibility, isLocked, onLayoutChange, togglePanel, resetLayout, toggleLock } =
    useGridLayout(containerWidth);

  useEffect(() => {
    return () => observerRef.current?.disconnect();
  }, []);

  const settingsBarHeight = 40;
  const availableHeight =
    typeof window !== "undefined" ? window.innerHeight - headerHeight - settingsBarHeight : 800;
  const rowHeight = Math.max(
    12,
    Math.floor((availableHeight - GRID_MARGIN[1] * (GRID_MAX_ROWS + 1)) / GRID_MAX_ROWS),
  );

  const onChangePairId = (pairSymbols: string, type: "spot" | "perps") => {
    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols },
      search: { type },
      replace: true,
    });
  };

  const onChangeAction = (action: "buy" | "sell") => {
    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols },
      replace: true,
      search: { order_type, action, type },
    });
  };

  const onChangeOrderType = (order_type: "limit" | "market") => {
    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols },
      replace: true,
      search: { order_type, action, type },
    });
  };

  const [baseSymbol, quoteSymbol] = pairSymbols.split("-");

  const pairId = {
    baseDenom: coins.bySymbol[baseSymbol]?.denom,
    quoteDenom: coins.bySymbol[quoteSymbol]?.denom,
  };

  return (
    <div
      className="flex flex-col w-full min-h-screen relative overflow-visible"
      style={{ minHeight: `calc(100vh - ${headerHeight}px)` }}
    >
      <ProTrade
        pairId={pairId}
        onChangePairId={onChangePairId}
        action={action}
        onChangeAction={onChangeAction}
        orderType={order_type}
        onChangeOrderType={onChangeOrderType}
        type={type}
      >
        {isLg ? (
          <div className="flex flex-col flex-1 bg-surface-secondary-rice" ref={containerRef}>
            {containerWidth > 0 && (
              <GridLayout
                className="flex-1"
                width={containerWidth}
                layout={fullLayout}
                gridConfig={{
                  cols: GRID_COLS,
                  rowHeight,
                  margin: GRID_MARGIN,
                  containerPadding: GRID_CONTAINER_PADDING,
                }}
                dragConfig={{ enabled: !isLocked, handle: ".grid-drag-handle" }}
                resizeConfig={{ enabled: !isLocked }}
                onLayoutChange={(newLayout) => onLayoutChange(newLayout)}
              >
                <div key="chart" className={visibility.chart ? "" : "!hidden"}>
                  <GridPanel panelId="chart" isLocked={isLocked} onClose={togglePanel}>
                    <ProTrade.Header
                      actions={
                        <SettingsDrawer
                          visibility={visibility}
                          isLocked={isLocked}
                          onTogglePanel={togglePanel}
                          onToggleLock={toggleLock}
                          onReset={resetLayout}
                        />
                      }
                    />
                    <ProTrade.Chart />
                  </GridPanel>
                </div>
                <div key="orderbook" className={visibility.orderbook ? "" : "!hidden"}>
                  <GridPanel panelId="orderbook" isLocked={isLocked} onClose={togglePanel}>
                    <ProTrade.OrderBook />
                  </GridPanel>
                </div>
                <div key="history" className={visibility.history ? "" : "!hidden"}>
                  <GridPanel panelId="history" isLocked={isLocked} onClose={togglePanel}>
                    <ProTrade.History />
                  </GridPanel>
                </div>
                <div key="trademenu" className={visibility.trademenu ? "" : "!hidden"}>
                  <GridPanel panelId="trademenu" isLocked={isLocked} onClose={togglePanel}>
                    <ProTrade.TradeMenu />
                  </GridPanel>
                </div>
              </GridLayout>
            )}
          </div>
        ) : (
          <>
            <div className="flex flex-col flex-1">
              <div className="flex flex-col flex-1">
                <div className="flex flex-col flex-1 justify-end">
                  <ProTrade.Header />
                  <ProTrade.Chart />
                </div>
                <ProTrade.OrderBook />
              </div>
              <ProTrade.History />
            </div>
            <div className="hidden lg:flex pt-4 w-full lg:w-[331px] xl:[width:clamp(279px,20vw,330px)] bg-surface-primary-rice shadow-account-card z-20 self-stretch">
              <ProTrade.TradeMenu />
            </div>
          </>
        )}
      </ProTrade>
    </div>
  );
}
