import type React from "react";
import { Button } from "../atoms/Button";
import { PortfolioTable } from "./PortfolioTable";

interface Props {
  sendAction: () => void;
  receiveAction: () => void;
}

export const SpotPortfolioTable: React.FC<Props> = ({ sendAction, receiveAction }) => {
  return (
    <PortfolioTable
      topComponent={
        <div className="flex flex-col gap-3 sm:flex-row w-full">
          <Button className="flex-1" onClick={sendAction}>
            Send
          </Button>
          <Button className="flex-1" onClick={receiveAction}>
            Receive
          </Button>
        </div>
      }
    />
  );
};
