import { useConfig, usePrices } from "@left-curve/store";

import { capitalize, formatNumber, formatUnits } from "@left-curve/dango/utils";
import { formatDistanceToNow } from "date-fns/formatDistanceToNow";
import { twMerge } from "#utils/twMerge.js";

import { AddressVisualizer } from "./AddressVisualizer";
import { Badge } from "./Badge";
import { TextCopy } from "./TextCopy";
import { IconLink } from "./icons/IconLink";

import type {
  Address,
  Directions,
  IndexedMessage,
  OneRequired,
  PairId,
  Prettify,
} from "@left-curve/dango/types";
import { format } from "date-fns";

import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { AnyCoin } from "@left-curve/store/types";
import type React from "react";
import type { PropsWithChildren } from "react";
import { Button } from "./Button";
import { PairAssets } from "./PairAssets";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type CellAssetProps = Prettify<
  {
    className?: string;
    noImage?: boolean;
  } & OneRequired<{ asset: AnyCoin; denom: string }, "asset", "denom">
>;

const Asset: React.FC<CellAssetProps> = ({ asset, noImage, denom }) => {
  const { coins, getCoinInfo } = useConfig();

  const coin = asset || getCoinInfo(denom as string);

  if (!coin) return <div className="flex h-full items-center diatype-sm-medium ">-</div>;

  return (
    <div className="flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto">
      {!noImage &&
        (coin.type === "lp" ? (
          <PairAssets assets={[coin.base, coin.quote]} />
        ) : (
          <img
            src={coin.logoURI}
            alt={coin.symbol}
            className="w-7 h-7 select-none drag-none"
            loading="lazy"
          />
        ))}
      <p className="min-w-fit">{coin.symbol}</p>
    </div>
  );
};

type CellAssetsProps = {
  className?: string;
  assets: AnyCoin[];
  noImage?: boolean;
};

const Assets: React.FC<CellAssetsProps> = ({ assets, noImage }) => {
  return (
    <div className="flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto">
      {!noImage && <PairAssets assets={assets} />}
      <p className="min-w-fit">
        {assets.map((asset, i) => (
          <span key={`text-${asset.symbol}-${i}`}>
            {asset.symbol}
            {i < assets.length - 1 ? "- " : ""}
          </span>
        ))}
      </p>
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
    <div className={twMerge("flex flex-col gap-1 diatype-sm-medium text-tertiary-500", className)}>
      <p>{formatUnits(amount, decimals)}</p>
      <p>{price}</p>
    </div>
  );
};

type CellTextProps = {
  className?: string;
  text: string | number;
};

const Text: React.FC<CellTextProps> = ({ text, className }) => {
  return (
    <div className={twMerge("flex flex-col gap-1 text-tertiary-500", className)}>
      <p>{text}</p>
    </div>
  );
};

type CellNumberProps = {
  className?: string;
  formatOptions: FormatNumberOptions;
  value: number | string;
};

const CellNumber: React.FC<CellNumberProps> = ({ value, formatOptions, className }) => {
  return (
    <div className={twMerge("flex flex-col gap-1 text-tertiary-500", className)}>
      <p>{formatNumber(value, formatOptions)}</p>
    </div>
  );
};

type CellOrderDirectionProps = {
  className?: string;
  direction: Directions;
  text: string;
};

const OrderDirection: React.FC<CellOrderDirectionProps> = ({ text, direction, className }) => {
  return (
    <div
      className={twMerge(
        "flex flex-col gap-1 diatype-xs-medium",
        direction === "ask" ? "text-status-fail" : "text-status-success",
        className,
      )}
    >
      <p>{text}</p>
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
        "flex h-full flex-col gap-1 diatype-sm-medium text-tertiary-500 my-auto justify-center",
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
      <p className="diatype-mono-sm-medium cursor-pointer" onClick={navigate}>
        {blockHeight}
      </p>
    </div>
  );
};

type CellAgeProps = {
  date: Date | string | number;
  addSuffix?: boolean;
};

const Age: React.FC<CellAgeProps> = ({ date, addSuffix }) => {
  return (
    <p className="h-full flex items-center min-w-32">{formatDistanceToNow(date, { addSuffix })}</p>
  );
};

type CellSenderProps = {
  sender: Address;
  navigate: (url: string) => void;
};

const Sender: React.FC<CellSenderProps> = ({ sender, navigate }) => {
  return (
    <div className="flex h-full items-center min-w-64">
      <AddressVisualizer address={sender} withIcon onClick={navigate} />
    </div>
  );
};

type CellTxResultProps = {
  isSuccess: boolean;
  text: string;
  className?: string;
  total: number;
};

const TxResult: React.FC<CellTxResultProps> = ({ className, isSuccess, text, total }) => {
  const color = isSuccess ? "green" : "red";

  return (
    <div className={twMerge("flex h-full items-center gap-1", className)}>
      <Badge text={text} color={color} />
      {total > 1 ? <Badge text={`+${total - 1}`} color={color} /> : null}
    </div>
  );
};

type CellTxHashProps = {
  hash: string;
  navigate?: () => void;
};

const TxHash: React.FC<CellTxHashProps> = ({ hash, navigate }) => {
  return (
    <div
      className="flex items-center h-full gap-1 cursor-pointer diatype-mono-sm-medium text-secondary-700"
      onClick={navigate}
    >
      <div className="flex items-center hover:text-primary-900">
        <p className="truncate max-w-36">{hash}</p>
        <IconLink className="h-4 w-4" />
      </div>
      <TextCopy copyText={hash} className="h-4 w-4 text-primary-gray hover:text-primary-900" />
    </div>
  );
};

type CellTimeProps = {
  className?: string;
  date: Date;
};

const Time: React.FC<CellTimeProps> = ({ date, className }) => {
  return (
    <div className={twMerge("flex flex-col gap-1 diatype-sm-medium text-tertiary-500", className)}>
      <p>{format(date, "MM/dd")}</p>
    </div>
  );
};

type CellActionProps = {
  classNames?: {
    cell?: string;
    button?: string;
  };
  isDisabled?: boolean;
  action: () => void;
  label: string;
};

const Action: React.FC<CellActionProps> = ({ action, label, classNames, isDisabled }) => {
  return (
    <div
      className={twMerge(
        "flex flex-col gap-1 diatype-sm-medium text-tertiary-500",
        classNames?.cell,
      )}
    >
      <Button
        variant="link"
        onClick={action}
        className={twMerge(classNames?.button)}
        isDisabled={isDisabled}
      >
        {label}
      </Button>
    </div>
  );
};

type CellTxMessagesProps = {
  messages: IndexedMessage[];
};

const TxMessages: React.FC<CellTxMessagesProps> = ({ messages }) => {
  const [firstMessage] = messages;
  const extraMessages = messages.length - 1;
  return (
    <div className="flex h-full items-center gap-1">
      <Badge text={capitalize(firstMessage.methodName)} color="blue" />
      {extraMessages ? <Badge text={`+${extraMessages}`} color="red" /> : null}
    </div>
  );
};

type CellPairNameProps = {
  pairId: PairId;
  type?: string;
  className?: string;
};

const PairName: React.FC<CellPairNameProps> = ({ pairId, type, className }) => {
  const { coins } = useConfig();
  const { baseDenom, quoteDenom } = pairId;
  const baseCoin = coins[baseDenom];
  const quoteCoin = coins[quoteDenom];

  return (
    <div
      className={twMerge(
        "flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto",
        className,
      )}
    >
      <p className="min-w-fit">{`${baseCoin.symbol}-${quoteCoin.symbol}`}</p>
      {type ? <Badge text={type} color="blue" size="s" /> : null}
    </div>
  );
};

export const Cell = Object.assign(Container, {
  Age,
  Asset,
  Assets,
  Action,
  Amount,
  Time,
  Sender,
  Text,
  TxHash,
  Number: CellNumber,
  OrderDirection,
  TxMessages,
  TxResult,
  MarketPrice,
  PairName,
  BlockHeight,
});
