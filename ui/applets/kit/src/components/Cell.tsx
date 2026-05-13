import { useConfig, useFavPairs, usePrices } from "@left-curve/store";

import { capitalize, formatUnits } from "@left-curve/dango/utils";
import { FormattedNumber } from "./FormattedNumber";
import { formatDistanceToNow } from "date-fns/formatDistanceToNow";
import { twMerge } from "@left-curve/foundation";

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
import { memo } from "react";
import type React from "react";
import type { PropsWithChildren } from "react";
import { Button } from "./Button";
import { PairAssets } from "./PairAssets";
import { IconStar } from "./icons/IconStar";
import { IconEmptyStar } from "./icons/IconEmptyStar";

const TokenImage = memo(({ src, alt }: { src?: string; alt: string }) => (
  <img src={src} alt={alt} className="w-5 h-5 flex-shrink-0" />
));

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
  const { coins } = useConfig();

  const coin = asset || coins.getCoinInfo(denom as string);

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
    <div
      className={twMerge("flex flex-col gap-1 diatype-sm-medium text-ink-tertiary-500", className)}
    >
      <p>{formatUnits(amount, decimals)}</p>
      <p>{price}</p>
    </div>
  );
};

type CellTextProps = {
  className?: string;
  text: React.ReactNode;
};

const Text: React.FC<CellTextProps> = ({ text, className }) => {
  return (
    <div className={twMerge("flex flex-col gap-1 text-ink-tertiary-500", className)}>
      <p>{text}</p>
    </div>
  );
};

type CellNumberProps = {
  className?: string;
  formatOptions?: Partial<FormatNumberOptions>;
  value: number | string;
};

const CellNumber: React.FC<CellNumberProps> = ({ value, formatOptions, className }) => {
  return (
    <div className={twMerge("flex flex-col gap-1 text-ink-tertiary-500", className)}>
      <FormattedNumber number={value} formatOptions={formatOptions} />
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
  formatOptions?: Partial<FormatNumberOptions>;
  denom: string;
};

const MarketPrice: React.FC<CellMarketPriceProps> = ({ denom, className, formatOptions }) => {
  const { prices = {} } = usePrices();
  const price = prices[denom] || {};

  return (
    <div
      className={twMerge(
        "flex h-full flex-col gap-1 diatype-sm-medium text-ink-tertiary-500 my-auto justify-center",
        className,
      )}
    >
      <FormattedNumber
        number={price.humanizedPrice || 0}
        formatOptions={{ ...formatOptions, currency: "usd" }}
      />
    </div>
  );
};

type CellBlockHeightProps = {
  blockHeight: number;
  href?: string;
  navigate: () => void;
};

const BlockHeight: React.FC<CellBlockHeightProps> = ({ blockHeight, href, navigate }) => {
  return (
    <div className="flex h-full items-center">
      <a
        href={href}
        className="diatype-mono-sm-medium cursor-pointer"
        onClick={(e) => {
          e.preventDefault();
          navigate();
        }}
      >
        {blockHeight}
      </a>
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
  href?: string;
  navigate?: () => void;
};

const TxHash: React.FC<CellTxHashProps> = ({ hash, href, navigate }) => {
  return (
    <div className="flex items-center h-full gap-1 diatype-mono-sm-medium text-ink-secondary-700">
      <a
        href={href}
        className="flex items-center cursor-pointer hover:text-ink-primary-900"
        onClick={(e) => {
          e.preventDefault();
          navigate?.();
        }}
      >
        <p className="truncate max-w-36">{hash}</p>
        <IconLink className="h-4 w-4" />
      </a>
      <TextCopy
        copyText={hash}
        className="h-4 w-4 text-ink-secondary-700 hover:text-ink-primary-900"
      />
    </div>
  );
};

type CellTimeProps = {
  className?: string;
  dateFormat: string;
  date: Date | string | number;
};

const Time: React.FC<CellTimeProps> = ({ date, dateFormat, className }) => {
  return (
    <div className={twMerge("flex flex-col gap-1", className)}>
      <p>{format(date, dateFormat)}</p>
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
        "flex flex-col gap-1 diatype-sm-medium text-ink-tertiary-500",
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
      {extraMessages ? <Badge text={`+${extraMessages}`} color="blue" /> : null}
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
  const baseCoin = coins.byDenom[baseDenom];
  const quoteCoin = coins.byDenom[quoteDenom];

  return (
    <div
      className={twMerge(
        "flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto min-w-fit pr-2",
        className,
      )}
    >
      <p className="whitespace-nowrap">{`${baseCoin.symbol}-${quoteCoin.symbol}`}</p>
      {type ? <Badge text={type} color="blue" size="s" /> : null}
    </div>
  );
};

type CellPairNameWithFavProps = {
  pairId: PairId;
  type?: string;
  className?: string;
};

const PairNameWithFav: React.FC<CellPairNameWithFavProps> = memo(({ pairId, type, className }) => {
  const { coins } = useConfig();
  const { baseDenom, quoteDenom } = pairId;
  const baseCoin = coins.byDenom[baseDenom];
  const quoteCoin = coins.byDenom[quoteDenom];
  const { toggleFavPair, hasFavPair } = useFavPairs();

  const pairSymbols = `${baseCoin.symbol}-${quoteCoin.symbol}`;

  const isFav = hasFavPair(pairSymbols);

  return (
    <div
      className={twMerge(
        "flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto min-w-fit pr-2",
        className,
      )}
    >
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          toggleFavPair(pairSymbols);
        }}
        className="focus:outline-none flex-shrink-0"
      >
        {isFav ? (
          <IconStar className="w-4 h-4 text-fg-primary-700" />
        ) : (
          <IconEmptyStar className="w-4 h-4 text-fg-primary-700" />
        )}
      </button>
      <TokenImage src={baseCoin.logoURI} alt={baseCoin.symbol} />
      <p className="whitespace-nowrap">{`${baseCoin.symbol}-${quoteCoin.symbol}`}</p>
      {type ? <Badge text={type} color="blue" size="s" /> : null}
    </div>
  );
});

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
  PairNameWithFav,
  BlockHeight,
});
