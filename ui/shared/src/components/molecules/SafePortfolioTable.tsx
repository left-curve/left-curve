import type React from "react";

import { Button } from "../atoms/Button";
import { PortfolioTable } from "./PortfolioTable";

import type { Account } from "@left-curve/types";

interface Props {
  account: Account;
}

export const SafePortfolioTable: React.FC<Props> = ({ account }) => {
  return (
    <PortfolioTable
      account={account}
      bottomComponent={
        <div className="flex flex-col w-full">
          <Button className="flex-1 min-h-11">New Transaction</Button>
        </div>
      }
    />
  );
};
