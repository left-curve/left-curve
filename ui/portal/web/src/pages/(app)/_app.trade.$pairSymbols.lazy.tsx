import { createLazyFileRoute } from "@tanstack/react-router";

import { useConfig } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";

import { ProTrade } from "~/components/dex/ProTrade";

import type { PairId } from "@left-curve/dango/types";
import { useLayoutEffect, useState } from "react";

export const Route = createLazyFileRoute("/(app)/_app/trade/$pairSymbols")({
  component: ProTradeApplet,
});

function ProTradeApplet() {
  const navigate = useNavigate();
  const { coins } = useConfig();
  const { pairSymbols } = Route.useParams();
  const { action = "buy", order_type = "market" } = Route.useSearch();
  const [headerHeight, setHeaderHeight] = useState(0);

  useLayoutEffect(() => {
    const headerEl = document.querySelector<HTMLElement>("header");
    if (!headerEl) return;

    setHeaderHeight(headerEl.offsetHeight);

    const observer = new ResizeObserver(([entry]) => {
      setHeaderHeight(entry.contentRect.height);
    });

    observer.observe(headerEl);
    return () => observer.disconnect();
  }, []);

  const onChangePairId = ({ baseDenom, quoteDenom }: PairId) => {
    const baseSymbol = coins.byDenom[baseDenom]?.symbol;
    const quoteSymbol = coins.byDenom[quoteDenom]?.symbol;

    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols: `${baseSymbol}-${quoteSymbol}` },
      replace: true,
    });
  };

  const onChangeAction = (action: "buy" | "sell") => {
    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols },
      replace: true,
      search: { order_type, action },
    });
  };

  const onChangeOrderType = (order_type: "limit" | "market") => {
    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols },
      replace: true,
      search: { order_type, action },
    });
  };

  const [baseSymbol, quoteSymbol] = pairSymbols.split("-");

  const pairId = {
    baseDenom: coins.bySymbol[baseSymbol]?.denom,
    quoteDenom: coins.bySymbol[quoteSymbol]?.denom,
  };

  return (
    <div
      className="flex w-full min-h-screen lg:min-h-[calc(100vh-112px)] relative overflow-visible"
      style={{
        minHeight: `calc(100vh - ${headerHeight}px)`,
      }}
    >
      <ProTrade
        pairId={pairId}
        onChangePairId={onChangePairId}
        action={action}
        onChangeAction={onChangeAction}
        orderType={order_type}
        onChangeOrderType={onChangeOrderType}
      >
        <div className="flex flex-col flex-1">
          <div className="flex flex-col xl:flex-row flex-1">
            <div className="flex flex-col flex-1 justify-end">
              <ProTrade.Header />
              <ProTrade.Chart />
            </div>
            <ProTrade.OrderBook />
          </div>
          <ProTrade.History />
        </div>
        <div className="hidden lg:flex pt-4 w-full lg:w-[331px] xl:[width:clamp(279px,20vw,330px)] lg:bg-surface-secondary-rice shadow-account-card z-20 max-h-[calc(100vh-76px)] md:sticky top-[76px]">
          <ProTrade.TradeMenu />
        </div>
      </ProTrade>
    </div>
  );
}
