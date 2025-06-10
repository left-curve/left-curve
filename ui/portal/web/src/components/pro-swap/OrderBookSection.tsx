import { Tabs, useMediaQuery } from "@left-curve/applets-kit";
import type React from "react";
import { useEffect, useState } from "react";
import { TradingViewChart } from "./TradingViewChart";
import { OrderBook } from "./OrderBook";
import { LifeTrades } from "./LifeTrades";

export const OrderBookSection: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"order book" | "trades" | "graph">("graph");

  const { isLg } = useMediaQuery();

  useEffect(() => {
    setActiveTab(isLg ? "order book" : "graph");
  }, [isLg]);

  return (
    <div className="p-4 shadow-card-shadow bg-rice-25 flex flex-col gap-2 lg:min-w-[25rem] min-h-[42rem] lg:min-h-[35.25rem]">
      <Tabs
        color="line-red"
        layoutId="tabs-order-history"
        selectedTab={activeTab}
        keys={isLg ? ["order book", "trades"] : ["graph", "order book", "trades"]}
        fullWidth
        onTabChange={(tab) => setActiveTab(tab as "order book" | "trades")}
      />
      {activeTab === "graph" && <TradingViewChart />}
      {activeTab === "order book" && <OrderBook />}
      {activeTab === "trades" && <LifeTrades />}
    </div>
  );
};
