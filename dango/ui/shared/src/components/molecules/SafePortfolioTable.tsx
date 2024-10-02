import type React from "react";
import { Button } from "../atoms/Button";
import { PortfolioTable } from "./PortfolioTable";

export const SafePortfolioTable: React.FC = () => {
  return (
    <PortfolioTable
      bottomComponent={
        <div className="flex flex-col w-full">
          <Button color="danger" className="flex-1 min-h-11 italic rounded-3xl">
            New Transaction
          </Button>
        </div>
      }
    />
  );
};
