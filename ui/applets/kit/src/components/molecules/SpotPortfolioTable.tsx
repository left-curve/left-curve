import type React from "react";
import { Button } from "../atoms/Button";
import { PortfolioTable } from "./PortfolioTable";

import { useAccount } from "@left-curve/react";
import type { Account } from "@left-curve/types";

interface Props {
  account: Account;
  sendAction: () => void;
  receiveAction: () => void;
}

export const SpotPortfolioTable: React.FC<Props> = ({ account, sendAction, receiveAction }) => {
  const { account: selectedAccount } = useAccount();
  const isCurrentAccount = selectedAccount?.address === account.address;
  return (
    <PortfolioTable
      account={account}
      topComponent={
        isCurrentAccount ? (
          <div className="flex flex-col gap-3 sm:flex-row w-full">
            <Button className="flex-1" onClick={sendAction}>
              Send
            </Button>
            <Button className="flex-1" onClick={receiveAction}>
              Receive
            </Button>
          </div>
        ) : null
      }
    />
  );
};
