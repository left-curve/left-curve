import { createFileRoute } from "@tanstack/react-router";

import { ProTrade } from "~/components/dex/ProTrade";

export const Route = createFileRoute("/(app)/_app/dex")({
  component: ProTradeApplet,
});

function ProTradeApplet() {
  return (
    <div className="flex w-full min-h-screen lg:min-h-[calc(100vh-76px)] relative overflow-visible">
      <ProTrade>
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
