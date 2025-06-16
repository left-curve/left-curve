import { createFileRoute } from "@tanstack/react-router";

import { ProTrade } from "~/components/dex/ProTrade";

export const Route = createFileRoute("/(app)/_app/dex")({
  component: ProTradeApplet,
});

function ProTradeApplet() {
  return (
    <div className="flex w-full min-h-screen lg:min-h-[calc(100vh-76px)]">
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
        <div className="hidden lg:flex pt-4 lg:max-w-[25rem] lg:bg-rice-25 w-full shadow-card-shadow relative z-20">
          <ProTrade.TradeMenu />
        </div>
      </ProTrade>
    </div>
  );
}
