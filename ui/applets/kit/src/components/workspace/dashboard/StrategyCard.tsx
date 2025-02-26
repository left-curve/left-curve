import type React from "react";

export const StrategyCard: React.FC = () => {
  return (
    <div className="relative p-4  min-h-[8.5rem] min-w-[17.375rem] bg-rice-50 shadow-card-shadow rounded-2xl overflow-hidden">
      <img
        src="/images/strategy-card/cocodrile.svg"
        alt=""
        className="absolute z-0 bottom-0 right-0 "
      />
      <div className="flex flex-col gap-2 justify-between z-10 w-full h-full relative">
        <div className="flex flex-col gap-2">
          <div className="flex gap-2 text-lg">
            <div className="flex">
              <img
                src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                alt=""
                className="h-6 w-6 rounded-full"
              />
              <img
                src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                alt=""
                className="h-6 w-6 -ml-1 rounded-full"
              />
            </div>
            <p>ETH-USD</p>
          </div>
          <div className="text-xs bg-green-bean-200 text-gray-500 py-1 px-2 rounded-[4px] h-fit w-fit">
            Stable Strategy
          </div>
        </div>
        <div className="p-2 rounded-xl bg-rice-100 flex items-center justify-between text-xs">
          <div className="flex gap-2">
            <span className="text-gray-500">APY</span>
            <span>17.72%</span>
          </div>
          <div className="flex gap-2">
            <span className="text-gray-500">TVL</span>
            <span>15.63%</span>
          </div>
        </div>
      </div>
    </div>
  );
};
