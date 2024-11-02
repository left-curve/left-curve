import type React from "react";
import { Button } from "../atoms/Button";
import { PortfolioTable } from "./PortfolioTable";

interface Props {
  navigate: (path: string) => void;
  sendUrl: string;
  receiveUrl: string;
}

export const SpotPortfolioTable: React.FC<Props> = ({ navigate, sendUrl, receiveUrl }) => {
  return (
    <PortfolioTable
      topComponent={
        <div className="flex flex-col gap-3 sm:flex-row w-full">
          <Button className="flex-1" onClick={() => navigate(sendUrl)}>
            Send
          </Button>
          <Button className="flex-1" onClick={() => navigate(receiveUrl)}>
            Receive
          </Button>
        </div>
      }
    />
  );
};
