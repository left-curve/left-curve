import { Badge, IconEmptyStar, IconShare } from "@left-curve/applets-kit";
import type React from "react";

export const PairHeader: React.FC = () => {
  return (
    <div className="flex bg-rice-50 p-4 gap-8 flex-col lg:flex-row w-full lg:justify-between">
      <div className="flex gap-8 items-center justify-between lg:items-start w-full lg:w-auto">
        <div className="flex lg:flex-col gap-2">
          <div className="flex gap-2 items-center">
            {/* This must be a pair, so the asset should be a <StackAssets /> component instead of the image */}
            <img
              src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
              alt=""
              className="h-7 w-7 drag-none select-none"
            />
            <p className="diatype-lg-heavy text-gray-700 min-w-fit">ETH-USDC</p>
          </div>
          <Badge text="Perp" color="green" />
        </div>
        <div className="flex gap-2 items-center">
          <IconEmptyStar className="w-5 h-5 text-gray-500" />
          <IconShare className="w-5 h-5 text-gray-500" />
        </div>
      </div>
      <div className="gap-2 lg:gap-4 grid grid-cols-1 lg:flex lg:flex-wrap">
        <div className="flex gap-1 flex-row lg:flex-col lg:items-start">
          <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">Mark</p>
          <p>83,565</p>
        </div>
        <div className="flex gap-1 flex-row lg:flex-col lg:items-start">
          <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">Last price</p>
          <p>$2,578</p>
        </div>
        <div className="flex gap-1 flex-row lg:flex-col lg:items-start">
          <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">Oracle</p>
          <p>83,565</p>
        </div>
        <div className="flex gap-1 flex-row lg:flex-col lg:items-start">
          <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">24h Change</p>
          <p className="text-red-bean-400">-542 / 0.70</p>
        </div>
        <div className="flex gap-1 flex-row lg:flex-col lg:items-start">
          <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">24h Volume</p>
          <p>$2,457,770,700.50</p>
        </div>
      </div>
    </div>
  );
};
