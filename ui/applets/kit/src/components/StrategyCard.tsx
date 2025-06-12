import type React from "react";
import { Button } from "#components/Button.js";

export const StrategyCard: React.FC = () => {
  return (
    <div className="relative p-4  min-h-[21.125rem] min-w-[17.375rem] bg-rice-50 shadow-account-card rounded-xl overflow-hidden">
      <img
        src="/images/strategy-card/cocodrile.svg"
        alt=""
        className="absolute z-0 bottom-0 right-0 "
      />
      <div className="flex flex-col gap-2 justify-between z-10 w-full h-full relative">
        <div className="flex flex-col gap-6 items-center justify-center text-center">
          <div className="flex">
            <img
              src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
              alt=""
              className="h-12 w-12 rounded-full"
            />
            <img
              src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
              alt=""
              className="h-12 w-12 -ml-1 rounded-full"
            />
          </div>
          <div className="flex flex-col gap-1">
            <p className="exposure-h3-italic">ETH Party!</p>
            <p className="diatype-lg-medium text-tertiary-500">
              Deposit <span className="font-bold">ETH-USDT</span>
            </p>
            <p className="diatype-lg-medium text-tertiary-500">Earn USDT</p>
          </div>
        </div>
        <div className="flex flex-col gap-4">
          <Button size="lg" variant="secondary" fullWidth>
            Select
          </Button>
          <div className="p-2 rounded-xl bg-rice-100/80 flex items-center justify-between">
            <div className="flex gap-2 items-center">
              <span className="text-tertiary-500 diatype-xs-medium">APY</span>
              <span className="text-gray-700 diatype-sm-bold">17.72%</span>
            </div>
            <div className="flex gap-2 items-center">
              <span className="text-tertiary-500 diatype-xs-medium">TVL</span>
              <span className="text-gray-700 diatype-sm-bold">15.63%</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
