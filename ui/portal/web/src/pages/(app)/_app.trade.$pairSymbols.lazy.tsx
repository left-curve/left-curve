import { createLazyFileRoute } from "@tanstack/react-router";

import { useConfig } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { useHeaderHeight } from "@left-curve/applets-kit";

import { ProTrade } from "~/components/dex/components/ProTrade";

export const Route = createLazyFileRoute("/(app)/_app/trade/$pairSymbols")({
  component: ProTradeApplet,
});

function ProTradeApplet() {
  const navigate = useNavigate();
  const { coins } = useConfig();
  const { pairSymbols } = Route.useParams();
  const { action = "buy", order_type = "market", type = "perps" } = Route.useSearch();
  const headerHeight = useHeaderHeight();

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
      className="flex w-full min-h-screen lg:min-h-[calc(100vh-112px)] relative overflow-visible lg:pb-[33px]"
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
        type={type}
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
