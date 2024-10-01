import type React from "react";
import { Button } from "../atoms/Button";
import { PortfolioTable } from "./PortfolioTable";

export const SpotPortfolioTable: React.FC = () => {
  return (
    <PortfolioTable
      topComponent={
        <div className="flex flex-col gap-3 sm:flex-row w-full">
          <Button color="danger" className="flex-1 min-h-11 italic rounded-3xl">
            Send
          </Button>
          <Button color="danger" className="flex-1 min-h-11 italic rounded-3xl">
            Receive
          </Button>
        </div>
      }
    />
  );
};
