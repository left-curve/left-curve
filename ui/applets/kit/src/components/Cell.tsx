import { usePrices } from "@left-curve/store";

import { formatNumber, formatUnits } from "@left-curve/dango/utils";
import { formatDistanceToNow } from "date-fns";
import { twMerge } from "#utils/twMerge.js";

import { AddressVisualizer } from "./AddressVisualizer";
import { Button } from "./Button";

import type { Address } from "@left-curve/dango/types";
import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { AnyCoin } from "@left-curve/store/types";
import type React from "react";
import type { PropsWithChildren } from "react";
import { Badge } from "./Badge";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
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

type CellBlockHeightProps = {
  blockHeight: number;
  navigate: () => void;
};

const BlockHeight: React.FC<CellBlockHeightProps> = ({ blockHeight, navigate }) => {
  return (
    <div className="flex h-full items-center">
      <Button variant="link" className="m-0 p-0 pr-1" onClick={navigate}>
        {blockHeight}
      </Button>
    </div>
  );
};

type CellAgeProps = {
  date: Date | string | number;
  addSuffix?: boolean;
};

const Age: React.FC<CellAgeProps> = ({ date, addSuffix }) => {
  return <p className="h-full flex items-center">{formatDistanceToNow(date, { addSuffix })}</p>;
};

type CellSenderProps = {
  sender: Address;
  navigate: (url: string) => void;
};

const Sender: React.FC<CellSenderProps> = ({ sender, navigate }) => {
  return (
    <div className="flex h-full items-center">
      <AddressVisualizer address={sender} withIcon onClick={navigate} />
    </div>
  );
};

type CellTxResultProps = {
  isSuccess: boolean;
  text: string;
  className?: string;
};

const TxResult: React.FC<CellTxResultProps> = ({ className, isSuccess, text }) => {
  const color = isSuccess ? "green" : "red";

  return (
    <div className={twMerge("flex h-full items-center", className)}>
      <Badge text={text} color={color} />
    </div>
  );
};

export const Cell = Object.assign(Container, {
  Age,
  Asset,
  Amount,
  Sender,
  TxResult,
  MarketPrice,
  BlockHeight,
});
