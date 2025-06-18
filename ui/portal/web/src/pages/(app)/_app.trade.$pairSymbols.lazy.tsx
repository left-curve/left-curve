import { useConfig } from "@left-curve/store";
import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";
import { coinsBySymbol } from "~/store";

import { ProTrade } from "~/components/dex/ProTrade";

import type { PairId } from "@left-curve/dango/types";

export const Route = createLazyFileRoute("/(app)/_app/trade/$pairSymbols")({
  component: ProTradeApplet,
});

function ProTradeApplet() {
  const navigate = useNavigate();
  const { coins } = useConfig();
  const { pairSymbols } = Route.useParams();

  const onChangePairId = ({ baseDenom, quoteDenom }: PairId) => {
    const baseSymbol = coins[baseDenom]?.symbol;
    const quoteSymbol = coins[quoteDenom]?.symbol;

    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols: `${baseSymbol}-${quoteSymbol}` },
      replace: false,
    });
  };

  const [baseSymbol, quoteSymbol] = pairSymbols.split("-");

  const pairId = {
    baseDenom: coinsBySymbol[baseSymbol]?.denom,
    quoteDenom: coinsBySymbol[quoteSymbol]?.denom,
  };

  return (
    <div className="flex w-full min-h-screen lg:min-h-[calc(100vh-76px)] relative overflow-visible">
      <ProTrade pairId={pairId} onChangePairId={onChangePairId}>
        <div className="flex flex-col flex-1">
          <div className="flex flex-col xl:flex-row flex-1">
            <div className="flex flex-col flex-1">
              <ProTrade.Header />
              <ProTrade.Chart />
            </div>
            <ProTrade.OrderBook />
          </div>
          <ProTrade.Orders />
        </div>
        <div className="hidden lg:flex pt-4 w-full lg:w-[331px] xl:[width:clamp(279px,20vw,422px)] lg:bg-rice-25 shadow-card-shadow z-20 max-h-[calc(100vh-76px)] md:sticky top-[76px]">
          <ProTrade.TradeMenu />
        </div>
      </ProTrade>
    </div>
  );
}
