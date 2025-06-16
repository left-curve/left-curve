import { StrategyCard } from "@left-curve/applets-kit";
import type React from "react";

export const StrategySection: React.FC = () => {
  return (
    <div className="flex gap-4 scrollbar-none justify-start lg:justify-between p-4 overflow-x-auto overflow-y-visible">
      <StrategyCard />
      <StrategyCard />
      <StrategyCard />
      <StrategyCard />
    </div>
  );
};
