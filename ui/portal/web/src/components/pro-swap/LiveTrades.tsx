import { IconLink, twMerge } from "@left-curve/applets-kit";
import type React from "react";
import { mockTrades } from "~/mock";

export const LiveTrades: React.FC = () => {
  return (
    <div className="flex gap-2 flex-col items-center justify-center ">
      <div className="diatype-xs-medium text-gray-500 w-full grid grid-cols-3 ">
        <p>Price</p>
        <p className="text-end">Size (ETH)</p>
        <p className="text-end">Time</p>
      </div>
      <div className="relative flex-1 w-full flex flex-col gap-1 items-center">
        {mockTrades.slice(0, 20).map((trade) => {
          return (
            <div
              key={trade.hash}
              className={
                "grid grid-cols-3 text-xs-medium text-gray-700 w-full cursor-pointer group relative"
              }
            >
              <p
                className={twMerge(
                  "z-10",
                  trade.side === "BUY" ? "text-status-success" : "text-status-fail",
                )}
              >
                {trade.price}
              </p>
              <p className="text-end z-10">{trade.size}</p>

              <div className="flex gap-1 items-center justify-end z-10">
                <p>{trade.createdAt}</p>
                <IconLink className="w-3 h-3" />
              </div>
              <span className="group-hover:bg-rice-50 h-[calc(100%+0.5rem)] w-[calc(100%+2rem)] absolute top-[-0.25rem] -left-4 z-0" />
            </div>
          );
        })}
      </div>
    </div>
  );
};
