import { Tabs } from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";

export const OpenOrder: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"open order" | "trade history">("open order");
  return (
    <div className="flex-1 p-4 bg-rice-25 flex flex-col gap-2 shadow-card-shadow">
      <div className="relative">
        <Tabs
          color="line-red"
          layoutId="tabs-open-order"
          selectedTab={activeTab}
          keys={["open order", "trade history"]}
          onTabChange={(tab) => setActiveTab(tab as "open order" | "trade history")}
        />
        <span className="w-full absolute h-[1px] bg-gray-100 bottom-[0.25rem]" />
      </div>
      TABLA
    </div>
  );
};
