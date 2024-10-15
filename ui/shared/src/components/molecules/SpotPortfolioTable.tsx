import type React from "react";
import { DangoButton } from "../atoms/DangoButton";
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
          <DangoButton className="flex-1" onClick={() => navigate(sendUrl)}>
            Send
          </DangoButton>
          <DangoButton className="flex-1" onClick={() => navigate(receiveUrl)}>
            Receive
          </DangoButton>
        </div>
      }
    />
  );
};
