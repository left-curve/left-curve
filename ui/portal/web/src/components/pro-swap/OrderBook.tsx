import { Tabs } from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";

export const OrderBook: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"order book" | "trades">("order book");
  return (
    <div className="p-4 shadow-card-shadow bg-rice-25 flex flex-col gap-2 min-w-[20rem]">
      <Tabs
        color="line-red"
        layoutId="tabs-order-history"
        selectedTab={activeTab}
        keys={["order book", "trades"]}
        fullWidth
        onTabChange={(tab) => setActiveTab(tab as "order book" | "trades")}
      />
    </div>
  );
};
