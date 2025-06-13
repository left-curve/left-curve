import { useMediaQuery } from "@left-curve/applets-kit";
import { createFileRoute } from "@tanstack/react-router";
import { TradeMenu } from "~/components/foundation/TradeMenu";
import { OpenOrder } from "~/components/pro-swap/OpenOrder";
import { OrderBookSection } from "~/components/pro-swap/OrderBookSection";
import { PairHeader } from "~/components/pro-swap/PairHeader";
import { TradingViewChart } from "~/components/pro-swap/TradingViewChart";

export const Route = createFileRoute("/(app)/_app/pro-swap")({
  component: RouteComponent,
});

function RouteComponent() {
  const { isLg } = useMediaQuery();
  return (
    <div className="flex w-full min-h-screen lg:min-h-[calc(100vh-76px)]">
      <div className="flex flex-col flex-1">
        <div className="flex flex-col xl:flex-row flex-1">
          <div className="flex flex-col flex-1">
            <PairHeader />
            {isLg && (
              <div className="shadow-card-shadow bg-rice-25">
                <TradingViewChart />
              </div>
            )}
          </div>
          <OrderBookSection />
        </div>
        <OpenOrder />
      </div>
      <div className="hidden lg:flex pt-4 lg:max-w-[25rem] lg:bg-rice-25 w-full shadow-card-shadow relative z-20">
        <TradeMenu.Menu />
      </div>
    </div>
  );
}
