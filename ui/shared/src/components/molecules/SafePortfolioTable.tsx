import type React from "react";

import { Button } from "../atoms/Button";
import { PortfolioTable } from "./PortfolioTable";

export const SafePortfolioTable: React.FC = () => {
  return (
    <PortfolioTable
      bottomComponent={
        <div className="flex flex-col w-full">
          <Button className="flex-1 min-h-11">New Transaction</Button>
        </div>
      }
    />
  );
};
