import { createFileRoute } from "@tanstack/react-router";

import { ProSwap } from "~/components/dex/ProSwap";

export const Route = createFileRoute("/(app)/_app/pro-swap")({
  component: ProSwapApplet,
});

function ProSwapApplet() {
  return (
    <div className="flex w-full min-h-screen lg:min-h-[calc(100vh-76px)]">
      <ProSwap>
        <div className="flex flex-col flex-1">
          <div className="flex flex-col xl:flex-row flex-1">
            <div className="flex flex-col flex-1">
              <ProSwap.Header />
              <ProSwap.Chart />
            </div>
            <ProSwap.OrderBook />
          </div>
          <ProSwap.Orders />
        </div>
        <div className="hidden lg:flex pt-4 lg:max-w-[25rem] lg:bg-rice-25 w-full shadow-card-shadow relative z-20">
          <ProSwap.TradeMenu />
        </div>
      </ProSwap>
    </div>
  );
}
