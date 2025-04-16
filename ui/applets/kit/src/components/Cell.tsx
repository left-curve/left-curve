import { formatNumber, formatUnits } from "@left-curve/dango/utils";
import { usePrices } from "@left-curve/store";
import { twMerge } from "#utils/twMerge.js";

import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { AnyCoin } from "@left-curve/store/types";
import type React from "react";
import type { PropsWithChildren } from "react";

const Root: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type CellAssetProps = {
  className?: string;
  asset: AnyCoin;
};

const Asset: React.FC<CellAssetProps> = ({ asset }) => {
  return (
    <div className="flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto">
      <img src={asset.logoURI} alt={asset.name} className="h-8 w-8" />
      <p className="min-w-fit">{asset.symbol}</p>
    </div>
  );
};

type CellAmountProps = {
  className?: string;
  price: string;
  amount: string;
  decimals: number;
};

const Amount: React.FC<CellAmountProps> = ({ amount, price, decimals, className }) => {
  return (
    <div className={twMerge("flex flex-col gap-1 diatype-sm-medium text-gray-500", className)}>
      <p>{formatUnits(amount, decimals)}</p>
      <p>{price}</p>
    </div>
  );
};

type CellMarketPriceProps = {
  className?: string;
  formatOptions: FormatNumberOptions;
  denom: string;
};
const MarketPrice: React.FC<CellMarketPriceProps> = ({ denom, className, formatOptions }) => {
  const { prices = {} } = usePrices();
  const price = prices[denom] || {};

  return (
    <div
      className={twMerge(
        "flex h-full flex-col gap-1 diatype-sm-medium text-gray-500 my-auto justify-center",
        className,
      )}
    >
      <p>{formatNumber(price.humanizedPrice || 0, { ...formatOptions, currency: "usd" })}</p>
    </div>
  );
};

export const Cell = Object.assign(Root, {
  Asset,
  Amount,
  MarketPrice,
});
