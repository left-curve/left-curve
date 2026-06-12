import { createLazyFileRoute } from "@tanstack/react-router";

import { useNavigate } from "@tanstack/react-router";
import { useHeaderHeight } from "@left-curve/applets-kit";
import { useCallback } from "react";
import { MarketPair } from "@left-curve/foundation/market-pair";

import { ProTrade } from "~/components/dex/components/ProTrade";

export const Route = createLazyFileRoute("/(app)/_app/trade/$ticker")({
  component: ProTradeApplet,
});

function ProTradeApplet() {
  const navigate = useNavigate();
  const { ticker } = Route.useParams();
  const { action = "buy", order_type = "market" } = Route.useSearch();
  const headerHeight = useHeaderHeight();

  const onChangeTicker = useCallback(
    (ticker: string) => {
      navigate({
        to: "/trade/$ticker",
        params: { ticker },
        replace: true,
      });
    },
    [navigate],
  );

  const onChangeAction = useCallback(
    (action: "buy" | "sell") => {
      navigate({
        to: "/trade/$ticker",
        params: { ticker },
        replace: true,
        search: { order_type, action },
      });
    },
    [navigate, order_type, ticker],
  );

  const onChangeOrderType = useCallback(
    (order_type: "limit" | "market") => {
      navigate({
        to: "/trade/$ticker",
        params: { ticker },
        replace: true,
        search: { order_type, action },
      });
    },
    [action, navigate, ticker],
  );

  const pair = MarketPair.tryFromTicker(ticker) ?? MarketPair.default;

  return (
    <div
      className="flex w-full min-h-screen lg:min-h-[calc(100vh-112px)] relative overflow-visible lg:pb-[33px]"
      style={{
        minHeight: `calc(100vh - ${headerHeight}px)`,
      }}
    >
      <ProTrade
        pair={pair}
        onChangeTicker={onChangeTicker}
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
        <div className="hidden lg:flex pt-4 w-full lg:w-[331px] xl:[width:clamp(279px,20vw,330px)] bg-surface-primary-rice shadow-account-card z-20 self-stretch">
          <ProTrade.TradeMenu />
        </div>
      </ProTrade>
    </div>
  );
}
