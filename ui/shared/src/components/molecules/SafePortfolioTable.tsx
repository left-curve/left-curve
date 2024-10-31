import type React from "react";

import { DangoButton } from "../atoms/DangoButton";
import { PortfolioTable } from "./PortfolioTable";

export const SafePortfolioTable: React.FC = () => {
  return (
    <PortfolioTable
      bottomComponent={
        <div className="flex flex-col w-full">
          <DangoButton className="flex-1 min-h-11">New Transaction</DangoButton>
        </div>
      }
    />
  );
};
