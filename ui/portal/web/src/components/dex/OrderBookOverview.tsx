import { FormattedNumber, Select, Spinner, useApp, useMediaQuery } from "@left-curve/applets-kit";
import { useEffect, useMemo, useState } from "react";
import { useRouter } from "@tanstack/react-router";

import { Direction, type PairId } from "@left-curve/dango/types";
import {
  liquidityDepthStore,
  useLiquidityDepthState,
  useLiveTradesState,
  useOrderBookState,
  type useProTradeState,
} from "@left-curve/store";
import { calculateTradeSize, Decimal, formatNumber, parseUnits } from "@left-curve/dango/utils";

import { IconLink, ResizerContainer, Tabs, twMerge, formatDate } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { AnyCoin } from "@left-curve/store/types";
import type { Controllers } from "@left-curve/applets-kit";

type OrderBookOverviewProps = {
  state: ReturnType<typeof useProTradeState>;
  controllers: Controllers;
};

export const OrderBookOverview: React.FC<OrderBookOverviewProps> = ({ state, controllers }) => {
  const [activeTab, setActiveTab] = useState<"order book" | "trades" | "graph">("graph");

  const { isLg } = useMediaQuery();

  useEffect(() => {
    setActiveTab(isLg ? "order book" : "graph");
  }, [isLg]);

  return (
    <ResizerContainer
      layoutId="order-book-section"
      className="overflow-hidden z-10 relative p-0 shadow-account-card bg-surface-primary-rice flex flex-col gap-2 w-full xl:[width:clamp(279px,20vw,330px)] min-h-[27.25rem] lg:min-h-[46.5rem] h-full"
    >
      <Tabs
        color="line-red"
        layoutId="tabs-order-history"
        selectedTab={activeTab}
        keys={isLg ? ["order book", "trades"] : ["graph", "order book", "trades"]}
        fullWidth
        onTabChange={(tab) => setActiveTab(tab as "order book" | "trades")}
        classNames={{ button: "exposure-xs-italic", base: "px-4 pt-4" }}
      />
      <div
        id="chart-container-mobile"
        className={twMerge("h-full w-full", { hidden: activeTab !== "graph" })}
      />
      {(activeTab === "trades" || activeTab === "order book") && (
        <>
          {activeTab === "order book" && <OrderBook state={state} controllers={controllers} />}
          {activeTab === "trades" && <LiveTrades state={state} controllers={controllers} />}
        </>
      )}
      <Subscription pairId={state.pairId} />
    </ResizerContainer>
  );
};

type OrderBookRowProps = {
  price: string;
  size: string;
  total: string;
  max: string;
  type: "bid" | "ask";
  onSelectPrice: (price: string) => void;
};

const OrderRow: React.FC<OrderBookRowProps> = (props) => {
  const { price, size, total, type, max, onSelectPrice } = props;
  const depthBarWidthPercent = Decimal(size).div(max).times(100).toFixed();

  const depthBarClass =
    type === "bid"
      ? "bg-utility-success-500 lg:right-auto right-0"
      : "bg-utility-error-300 opacity-[18%] lg:right-auto";

  return (
    <div className="relative diatype-xs-medium text-ink-secondary-700 grid grid-cols-2 lg:grid-cols-3 px-4 min-h-[23px] items-center">
      <div
        className={twMerge("absolute top-0 bottom-0 opacity-20 z-0", depthBarClass)}
        style={{ width: `${depthBarWidthPercent}%` }}
      />
      <div
        className={twMerge(
          "z-10 cursor-pointer leading-[8px]",
          type === "bid"
            ? "text-utility-success-600 text-end lg:text-left lg:order-none order-2"
            : "text-utility-error-600 lg:order-none lg:text-left",
        )}
        onClick={() => onSelectPrice(price)}
      >
        <FormattedNumber number={price} formatOptions={{ minimumTotalDigits: 10 }} />
      </div>
      <div className="z-10 justify-end text-end hidden lg:flex gap-1">
        <FormattedNumber number={size} formatOptions={{ minimumTotalDigits: 8 }} />
      </div>
      <div
        className={twMerge(
          "z-10",
          type === "bid" ? "text-start lg:text-end" : "order-1 lg:order-none text-end",
        )}
      >
        <FormattedNumber number={total} formatOptions={{ minimumTotalDigits: 8 }} />
      </div>
    </div>
  );
};

const OrderBook: React.FC<OrderBookOverviewProps> = ({ state, controllers }) => {
  const { baseCoin, quoteCoin, pair, pairId, bucketRecords, bucketSize, setBucketSize } = state;

  const bucketSizeCoin = liquidityDepthStore((s) => s.bucketSizeCoin);
  const setBucketSizeCoin = liquidityDepthStore((s) => s.setBucketSizeCoin);

  const bucketSizeSymbol = bucketSizeCoin === "base" ? baseCoin.symbol : quoteCoin.symbol;

  return (
    <div className="flex gap-2 flex-col items-center justify-center h-full">
      <div className="flex items-center justify-between w-full px-4">
        <Select value={bucketSize} onChange={(key) => setBucketSize(key)} variant="plain">
          {pair.params.bucketSizes.map((size) => {
            return (
              <Select.Item key={`bucket-${size}`} value={size}>
                {Decimal(size)
                  .mul(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
                  .toString()}
              </Select.Item>
            );
          })}
        </Select>
        <Select
          value={bucketSizeCoin === "base" ? baseCoin.symbol : quoteCoin.symbol}
          onChange={(key) => setBucketSizeCoin(key === baseCoin.symbol ? "base" : "quote")}
          variant="plain"
          classNames={{ listboxWrapper: "right-0 left-auto" }}
        >
          <Select.Item value={baseCoin.symbol}>{baseCoin.symbol}</Select.Item>
          <Select.Item value={quoteCoin.symbol}>{quoteCoin.symbol}</Select.Item>
        </Select>
      </div>
      <div className="diatype-xs-medium text-ink-tertiary-500 w-full grid grid-cols-4 lg:grid-cols-3 gap-2 px-4">
        <p className="order-2 lg:order-none text-end lg:text-start">
          {m["dex.protrade.history.price"]()}
        </p>
        <p className="hidden lg:block lg:order-none text-end">
          {m["dex.protrade.history.size"]({ symbol: bucketSizeSymbol })}
        </p>
        <p className=" order-1 lg:order-none lg:text-end">
          {m["dex.protrade.history.total"]({ symbol: bucketSizeSymbol })}
        </p>
        <p className="order-3 lg:hidden">{m["dex.protrade.history.price"]()}</p>
        <p className="order-4 text-end lg:order-none lg:hidden">
          {m["dex.protrade.history.total"]({ symbol: bucketSizeSymbol })}
        </p>
      </div>
      <LiquidityDepth
        pairId={pairId}
        bucketSize={bucketSize}
        bucketRecords={bucketRecords}
        base={baseCoin}
        quote={quoteCoin}
        onSelectPrice={(price) => controllers.setValue("price", price)}
      />
    </div>
  );
};

const LiveTrades: React.FC<OrderBookOverviewProps> = ({ state }) => {
  const { navigate } = useRouter();
  const { settings } = useApp();
  const { timeFormat } = settings;
  const { baseCoin, quoteCoin, pairId } = state;
  const { liveTradesStore } = useLiveTradesState({ pairId, subscribe: true });

  const trades = liveTradesStore((s) => s.trades);

  return (
    <div className="flex gap-2 flex-col items-center justify-start lg:max-h-[43rem] overflow-y-scroll scrollbar-none overflow-x-hidden relative px-4">
      <div className="diatype-xs-medium text-ink-tertiary-500 w-full grid grid-cols-3 sticky top-0 bg-surface-primary-rice z-20">
        <p>{m["dex.protrade.history.price"]()}</p>
        <p className="text-center">{m["dex.protrade.history.size"]({ symbol: baseCoin.symbol })}</p>
        <p className="text-end">{m["dex.protrade.history.time"]()}</p>
      </div>
      <div className="relative flex-1 w-full flex flex-col gap-1 items-center">
        {trades.map((trade, index) => (
          <div
            key={`${trade.addr}-${trade.createdAt}-${index}`}
            onClick={() => navigate({ to: `/block/${trade.blockHeight}` })}
            className={
              "grid grid-cols-3 diatype-xs-medium text-ink-secondary-700 w-full cursor-pointer group relative"
            }
          >
            <div
              className={twMerge(
                "z-10",
                trade.direction === Direction.Buy
                  ? "text-utility-success-600"
                  : "text-utility-error-600",
              )}
            >
              <FormattedNumber
                number={parseUnits(
                  trade.clearingPrice,
                  baseCoin.decimals - quoteCoin.decimals,
                  true,
                )}
                formatOptions={{ minimumTotalDigits: 8 }}
              />
            </div>
            <div className="text-center z-10 flex gap-1 justify-center">
              <FormattedNumber
                number={calculateTradeSize(trade, baseCoin.decimals).toFixed()}
                formatOptions={{ maximumTotalDigits: 5, minimumTotalDigits: 5 }}
              />
            </div>

            <div className="flex flex-nowrap whitespace-nowrap gap-1 items-center justify-end z-10">
              <p>{formatDate(trade.createdAt, timeFormat.replace("mm", "mm:ss"))}</p>
              <IconLink className="w-3 h-3 min-h-3 min-w-3" />
            </div>
            <span className="group-hover:bg-surface-tertiary-rice h-[calc(100%+0.5rem)] w-[calc(100%+2rem)] absolute top-[-0.25rem] -left-4 z-0" />
          </div>
        ))}
      </div>
    </div>
  );
};

type LiquidityDepthProps = {
  pairId: PairId;
  bucketSize: string;
  bucketRecords: number;
  base: AnyCoin;
  quote: AnyCoin;
  onSelectPrice: (price: string) => void;
};

const LiquidityDepth: React.FC<LiquidityDepthProps> = ({
  pairId,
  bucketSize,
  bucketRecords,
  base,
  quote,
  onSelectPrice,
}) => {
  const { isLg } = useMediaQuery();
  const { liquidityDepthStore } = useLiquidityDepthState({
    subscribe: true,
    pairId,
    bucketSize,
    bucketRecords,
  });

  const { liquidityDepth } = liquidityDepthStore();

  if (!liquidityDepth) return <Spinner fullContainer size="md" color="pink" />;

  const { bids, asks } = liquidityDepth;

  const asksOrdered = isLg ? [...asks.records].reverse() : [...asks.records];

  const max = Decimal.max(bids.highestSize, asks.highestSize).toFixed();

  return (
    <div className="flex-1 h-full flex gap-2 lg:flex-col items-start justify-center w-full">
      <div className="asks-container flex flex-1 flex-col w-full gap-[2px] order-2 lg:order-1 lg:justify-end">
        {asksOrdered.map((ask, i) => (
          <OrderRow
            key={`ask-${ask.price}-${i}`}
            type="ask"
            {...ask}
            max={max}
            onSelectPrice={onSelectPrice}
          />
        ))}
      </div>

      <Spread pairId={pairId} base={base} quote={quote} />

      <div className="bid-container flex flex-1 flex-col w-full gap-[2px] order-1 lg:order-3">
        {[...bids.records].map((bid, i) => (
          <OrderRow
            key={`bid-${bid.price}-${i}`}
            type="bid"
            {...bid}
            max={max}
            onSelectPrice={onSelectPrice}
          />
        ))}
      </div>
    </div>
  );
};

type SpreadProps = {
  pairId: PairId;
  base: AnyCoin;
  quote: AnyCoin;
};

const Spread: React.FC<SpreadProps> = ({ pairId, base, quote }) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { orderBookStore } = useOrderBookState({ pairId });
  const previousPrice = orderBookStore((s) => s.previousPrice);
  const currentPrice = orderBookStore((s) => s.currentPrice);
  const orderBook = orderBookStore((s) => s.orderBook);

  const spreadCalc = useMemo(() => {
    if (!orderBook?.bestAskPrice || !orderBook?.bestBidPrice || !orderBook?.midPrice) return null;
    const spread = Decimal(orderBook.bestAskPrice).minus(orderBook.bestBidPrice);
    const spreadPercent = spread.div(orderBook.midPrice).times(100);
    return { spread, spreadPercent };
  }, [orderBook]);

  return (
    <div className="hidden lg:flex w-full py-1 items-center justify-between relative order-2 px-4">
      <p
        className={twMerge(
          "diatype-m-bold relative z-20",
          Decimal(previousPrice).lte(currentPrice)
            ? "text-utility-error-600"
            : "text-utility-success-600",
        )}
      >
        {formatNumber(currentPrice || "0", formatNumberOptions)}
      </p>
      <div className="flex flex-col items-end text-ink-tertiary-500 relative z-20">
        <p className="diatype-xxs-medium">{m["dex.protrade.spread"]()}</p>
        <p className="diatype-xxs-medium">
          {!spreadCalc
            ? "n/a"
            : `${formatNumber(+spreadCalc.spread.mul(Decimal(10).pow(base.decimals - quote.decimals)).toFixed(), formatNumberOptions)} (${formatNumber(spreadCalc.spreadPercent.toFixed(), formatNumberOptions)}%)`}
        </p>
      </div>
      <span className="bg-surface-tertiary-rice w-[calc(100%+2rem)] absolute -left-4 top-0 h-full z-10" />
    </div>
  );
};

type SubscriptionProps = {
  pairId: PairId;
};
const Subscription: React.FC<SubscriptionProps> = ({ pairId }) => {
  useOrderBookState({ pairId, subscribe: true });
  return null;
};
