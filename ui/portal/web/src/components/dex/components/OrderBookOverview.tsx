import { FormattedNumber, Select, Spinner, useApp, useMediaQuery } from "@left-curve/applets-kit";
import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "@tanstack/react-router";

import { Direction, type PairId } from "@left-curve/dango/types";
import {
  liquidityDepthStore,
  useLiquidityDepthState,
  livePerpsTradesStore,
  useOrderBookState,
  useCurrentPrice,
  useTradeCoins,
  useAppConfig,
  perpsLiquidityDepthStore,
  useStorage,
  TradePairStore,
  liveSpotTradesStore,
  usePerpsLiquidityDepth,
} from "@left-curve/store";
import {
  bucketSizeToFractionDigits,
  calculateTradeSize,
  Decimal,
  formatNumber,
  parseUnits,
} from "@left-curve/dango/utils";

import { IconLink, ResizerContainer, Tabs, twMerge, formatDate } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { AnyCoin } from "@left-curve/store/types";
import type { Controllers } from "@left-curve/applets-kit";

type OrderBookOverviewProps = {
  controllers: Controllers;
};

export const OrderBookOverview: React.FC<OrderBookOverviewProps> = ({ controllers }) => {
  const [activeTab, setActiveTab] = useState<"order book" | "trades" | "graph">("graph");

  const { isLg } = useMediaQuery();

  const mode = TradePairStore((s) => s.mode);
  const pairId = TradePairStore((s) => s.pairId);
  const getPerpsPairId = TradePairStore((s) => s.getPerpsPairId);
  const { data: appConfig } = useAppConfig();

  const { baseCoin, quoteCoin } = useTradeCoins();

  const pairInfo =
    mode === "spot" ? appConfig.pairs[pairId.baseDenom] : appConfig.perpsPairs[getPerpsPairId()];

  const bucketSizes: string[] =
    pairInfo && "params" in pairInfo ? pairInfo.params.bucketSizes : pairInfo.bucketSizes;

  const [bucketSize, setBucketSize] = useState(bucketSizes[0]);

  useEffect(() => {
    setBucketSize(bucketSizes[0]);
  }, [bucketSizes]);

  usePerpsLiquidityDepth({
    pairId: getPerpsPairId(),
    bucketSize,
    subscribe: mode === "perps",
  });

  useEffect(() => {
    setActiveTab(isLg ? "order book" : "graph");
  }, [isLg]);

  const tabsKeys = useMemo(() => {
    return isLg ? ["order book", "trades"] : ["graph", "order book", "trades"];
  }, [isLg]);

  const bucketRecords = isLg ? 10 : 16;

  return (
    <ResizerContainer
      layoutId="order-book-section"
      className={twMerge("overflow-hidden z-10 relative p-0 flex flex-col gap-2 w-full h-full")}
    >
      <Tabs
        color="line-red"
        layoutId="tabs-order-history"
        selectedTab={activeTab}
        keys={tabsKeys}
        fullWidth
        onTabChange={(tab) => setActiveTab(tab as "order book" | "trades")}
        classNames={{ button: "exposure-xs-italic", base: "px-4 pt-3" }}
      />
      <div
        id="chart-container-mobile"
        className={twMerge("h-full w-full", { hidden: activeTab !== "graph" })}
      />
      {(activeTab === "trades" || activeTab === "order book") && (
        <>
          {activeTab === "order book" && (
            <OrderBook
              baseCoin={baseCoin}
              quoteCoin={quoteCoin}
              bucketSizes={bucketSizes}
              bucketSize={bucketSize}
              setBucketSize={setBucketSize}
              bucketRecords={bucketRecords}
              pairId={pairId}
              controllers={controllers}
              mode={mode}
            />
          )}
          {activeTab === "trades" && (
            <LiveTrades baseCoin={baseCoin} quoteCoin={quoteCoin} mode={mode} />
          )}
        </>
      )}
    </ResizerContainer>
  );
};

type OrderBookRowProps = {
  price: string;
  size: string;
  total: string;
  max: string;
  type: "bid" | "ask";
  priceFractionDigits: number;
  onSelectPrice: (price: string) => void;
  flashKey?: number;
};

const OrderRow: React.FC<OrderBookRowProps> = (props) => {
  const { price, size, total, type, max, priceFractionDigits, onSelectPrice, flashKey } = props;
  const depthBarWidthPercent = Decimal(size).div(max).times(100).toFixed();

  const depthBarClass =
    type === "bid"
      ? "bg-utility-success-500 opacity-10 lg:right-auto right-0"
      : "bg-utility-error-300 opacity-[18%] lg:right-auto";

  return (
    <div className="relative diatype-xs-medium text-ink-secondary-700 grid grid-cols-2 lg:grid-cols-3 px-4 min-h-[19px] items-center">
      {flashKey ? (
        <div
          key={flashKey}
          className={twMerge(
            "absolute inset-0 z-[1] pointer-events-none",
            type === "bid" ? "animate-flash-bid" : "animate-flash-ask",
          )}
        />
      ) : null}
      <div
        className={twMerge("absolute top-0 bottom-0 z-0", depthBarClass)}
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
        <FormattedNumber
          number={price}
          formatOptions={{ fractionDigits: priceFractionDigits }}
          tabular
        />
      </div>
      <div className="z-10 justify-end text-end hidden lg:flex gap-1">
        <FormattedNumber number={size} tabular />
      </div>
      <div
        className={twMerge(
          "z-10",
          type === "bid" ? "text-start lg:text-end" : "order-1 lg:order-none text-end",
        )}
      >
        <FormattedNumber number={total} tabular />
      </div>
    </div>
  );
};

type OrderBookProps = {
  baseCoin: AnyCoin & { amount: string };
  quoteCoin: AnyCoin & { amount: string };
  bucketSizes: string[];
  bucketSize: string;
  setBucketSize: (size: string) => void;
  bucketRecords: number;
  pairId: PairId;
  controllers: Controllers;
  mode: "spot" | "perps";
};

const OrderBook: React.FC<OrderBookProps> = ({
  baseCoin,
  quoteCoin,
  bucketSizes,
  bucketSize,
  setBucketSize,
  bucketRecords,
  pairId,
  controllers,
  mode,
}) => {
  const bucketSizeCoin = liquidityDepthStore((s) => s.bucketSizeCoin);
  const setBucketSizeCoin = liquidityDepthStore((s) => s.setBucketSizeCoin);
  const [perpsDisplayModeRaw, setPerpsDisplayMode] = useStorage<"base" | "quote">(
    "perps-order-book-display-mode",
    { initialValue: "base" },
  );
  const perpsDisplayMode = perpsDisplayModeRaw === "quote" ? "quote" : "base";

  const bucketSizeSymbol =
    mode === "perps"
      ? perpsDisplayMode === "quote"
        ? "USD"
        : baseCoin.symbol
      : bucketSizeCoin === "base"
        ? baseCoin.symbol
        : quoteCoin.symbol;

  const priceFractionDigits = useMemo(() => {
    const displaySize =
      mode === "perps"
        ? bucketSize
        : Decimal(bucketSize)
            .mul(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
            .toString();
    return bucketSizeToFractionDigits(displaySize);
  }, [bucketSize, mode, baseCoin.decimals, quoteCoin.decimals]);

  return (
    <div className="flex gap-2 flex-col items-center justify-center h-full">
      <div className="flex items-center justify-between w-full px-4">
        <Select value={bucketSize} onChange={(key) => setBucketSize(key)} variant="plain">
          {bucketSizes.map((size: string) => {
            const displaySize =
              mode === "perps"
                ? size
                : Decimal(size)
                    .mul(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
                    .toString();
            return (
              <Select.Item key={`bucket-${size}`} value={size}>
                {displaySize}
              </Select.Item>
            );
          })}
        </Select>
        {mode === "spot" && (
          <Select
            value={bucketSizeCoin === "base" ? baseCoin.symbol : quoteCoin.symbol}
            onChange={(key) => setBucketSizeCoin(key === baseCoin.symbol ? "base" : "quote")}
            variant="plain"
            classNames={{ listboxWrapper: "right-0 left-auto" }}
          >
            <Select.Item value={baseCoin.symbol}>{baseCoin.symbol}</Select.Item>
            <Select.Item value={quoteCoin.symbol}>{quoteCoin.symbol}</Select.Item>
          </Select>
        )}
        {mode === "perps" && (
          <Select
            value={perpsDisplayMode}
            onChange={(key) => setPerpsDisplayMode(key as "base" | "quote")}
            variant="plain"
            classNames={{ listboxWrapper: "right-0 left-auto" }}
          >
            <Select.Item value="base">{baseCoin.symbol}</Select.Item>
            <Select.Item value="quote">USD</Select.Item>
          </Select>
        )}
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
        mode={mode}
        displayMode={perpsDisplayMode}
        priceFractionDigits={priceFractionDigits}
      />
    </div>
  );
};

type LiveTradesProps = {
  baseCoin: AnyCoin & { amount: string };
  quoteCoin: AnyCoin & { amount: string };
  mode: "spot" | "perps";
};

const LiveTrades: React.FC<LiveTradesProps> = (props) => {
  return props.mode === "perps" ? <PerpsLiveTrades {...props} /> : <SpotLiveTrades {...props} />;
};

const PerpsLiveTrades: React.FC<LiveTradesProps> = ({ baseCoin }) => {
  const { navigate } = useRouter();
  const { settings } = useApp();
  const { timeFormat } = settings;

  const livePerps = livePerpsTradesStore((s) => s.trades);
  const perpsTrades = useDeferredValue(livePerps);

  return (
    <div className="flex gap-2 flex-col items-center justify-start flex-1 overflow-y-auto scrollbar-none overflow-x-hidden relative px-4">
      <div className="diatype-xs-medium text-ink-tertiary-500 w-full grid grid-cols-3 sticky top-0 bg-surface-primary-rice z-20">
        <p>{m["dex.protrade.history.price"]()}</p>
        <p className="text-center">{m["dex.protrade.history.size"]({ symbol: baseCoin.symbol })}</p>
        <p className="text-end">{m["dex.protrade.history.time"]()}</p>
      </div>
      <div className="relative flex-1 w-full flex flex-col gap-1 items-center">
        {perpsTrades.length ? (
          perpsTrades.map((trade, index) => {
            const isLong = Number(trade.fillSize) > 0;
            return (
              <div
                key={`${trade.orderId}-${trade.tradeIdx}-${index}`}
                onClick={() => navigate({ to: `/block/${trade.blockHeight}` })}
                className="grid grid-cols-3 diatype-xs-medium text-ink-secondary-700 w-full cursor-pointer group relative"
              >
                <div
                  className={twMerge(
                    "z-10",
                    isLong ? "text-utility-success-600" : "text-utility-error-600",
                  )}
                >
                  <FormattedNumber number={trade.fillPrice} tabular />
                </div>
                <div className="text-center z-10 flex gap-1 justify-center">
                  <FormattedNumber number={Math.abs(Number(trade.fillSize)).toString()} tabular />
                </div>
                <div className="flex flex-nowrap whitespace-nowrap gap-1 items-center justify-end z-10">
                  <p>{formatDate(trade.createdAt, timeFormat.replace("mm", "mm:ss"))}</p>
                  <IconLink className="w-3 h-3 min-h-3 min-w-3" />
                </div>
                <span className="group-hover:bg-surface-tertiary-rice h-[calc(100%+0.5rem)] w-[calc(100%+2rem)] absolute top-[-0.25rem] -left-4 z-0" />
              </div>
            );
          })
        ) : (
          <Spinner fullContainer size="md" color="pink" />
        )}
      </div>
    </div>
  );
};

const SpotLiveTrades: React.FC<LiveTradesProps> = ({ baseCoin, quoteCoin }) => {
  const { navigate } = useRouter();
  const { settings } = useApp();
  const { timeFormat } = settings;

  const liveTrades = liveSpotTradesStore((s) => s.trades);
  const trades = useDeferredValue(liveTrades);

  return (
    <div className="flex gap-2 flex-col items-center justify-start flex-1 overflow-y-auto scrollbar-none overflow-x-hidden relative px-4">
      <div className="diatype-xs-medium text-ink-tertiary-500 w-full grid grid-cols-3 sticky top-0 bg-surface-primary-rice z-20">
        <p>{m["dex.protrade.history.price"]()}</p>
        <p className="text-center">{m["dex.protrade.history.size"]({ symbol: baseCoin.symbol })}</p>
        <p className="text-end">{m["dex.protrade.history.time"]()}</p>
      </div>
      <div className="relative flex-1 w-full flex flex-col gap-1 items-center">
        {trades.length ? (
          trades.map((trade, index) => (
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
                  tabular
                />
              </div>
              <div className="text-center z-10 flex gap-1 justify-center">
                <FormattedNumber
                  number={calculateTradeSize(trade, baseCoin.decimals).toFixed()}
                  tabular
                />
              </div>

              <div className="flex flex-nowrap whitespace-nowrap gap-1 items-center justify-end z-10">
                <p>{formatDate(trade.createdAt, timeFormat.replace("mm", "mm:ss"))}</p>
                <IconLink className="w-3 h-3 min-h-3 min-w-3" />
              </div>
              <span className="group-hover:bg-surface-tertiary-rice h-[calc(100%+0.5rem)] w-[calc(100%+2rem)] absolute top-[-0.25rem] -left-4 z-0" />
            </div>
          ))
        ) : (
          <Spinner fullContainer size="md" color="pink" />
        )}
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
  mode: "spot" | "perps";
  displayMode: "base" | "quote";
  priceFractionDigits: number;
};

const LiquidityDepth: React.FC<LiquidityDepthProps> = ({
  pairId,
  bucketSize,
  bucketRecords,
  base,
  quote,
  onSelectPrice,
  mode,
  displayMode,
  priceFractionDigits,
}) => {
  const { isLg } = useMediaQuery();
  const { liquidityDepthStore } = useLiquidityDepthState({
    subscribe: mode === "spot",
    pairId,
    bucketSize,
    bucketRecords,
  });

  const spotDepth = liquidityDepthStore();
  const perpsDepthData = perpsLiquidityDepthStore((s) => s.liquidityDepth);

  const liquidityDepth = useMemo(() => {
    if (mode === "spot") return spotDepth.liquidityDepth;
    if (!perpsDepthData) return null;
    return perpsLiquidityDepthMapper(perpsDepthData, bucketRecords, displayMode);
  }, [mode, spotDepth.liquidityDepth, perpsDepthData, bucketRecords, displayMode]);

  const prevSizesRef = useRef<Map<string, string>>(new Map());
  const flashCountersRef = useRef<Map<string, number>>(new Map());

  const flashKeys = useMemo(() => {
    if (!liquidityDepth) return new Map<string, number>();
    const prev = prevSizesRef.current;
    const counters = flashCountersRef.current;
    const allRecords = [...liquidityDepth.bids.records, ...liquidityDepth.asks.records];

    for (const record of allRecords) {
      const prevSize = prev.get(record.price);
      if (prevSize !== undefined && prevSize !== record.size) {
        counters.set(record.price, (counters.get(record.price) ?? 0) + 1);
      }
    }

    const newPrev = new Map<string, string>();
    for (const record of allRecords) {
      newPrev.set(record.price, record.size);
    }
    prevSizesRef.current = newPrev;

    return new Map(counters);
  }, [liquidityDepth]);

  if (!liquidityDepth) return <Spinner fullContainer size="md" color="pink" />;

  const { bids, asks } = liquidityDepth;

  const asksOrdered = isLg ? [...asks.records].reverse() : [...asks.records];

  const max = Decimal.max(bids.highestSize, asks.highestSize).toFixed();

  return (
    <div className="flex-1 h-full flex gap-2 lg:flex-col items-start justify-center w-full">
      <div className="asks-container flex flex-1 flex-col w-full gap-[6px] order-2 lg:order-1 lg:justify-end">
        {asksOrdered.map((ask, i) => (
          <OrderRow
            key={`ask-${ask.price}-${i}`}
            type="ask"
            {...ask}
            max={max}
            priceFractionDigits={priceFractionDigits}
            onSelectPrice={onSelectPrice}
            flashKey={flashKeys.get(ask.price)}
          />
        ))}
      </div>

      <Spread pairId={pairId} base={base} quote={quote} mode={mode} />

      <div className="bid-container flex flex-1 flex-col w-full gap-[6px] order-1 lg:order-3">
        {[...bids.records].map((bid, i) => (
          <OrderRow
            key={`bid-${bid.price}-${i}`}
            type="bid"
            {...bid}
            max={max}
            priceFractionDigits={priceFractionDigits}
            onSelectPrice={onSelectPrice}
            flashKey={flashKeys.get(bid.price)}
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
  mode: "spot" | "perps";
};

const Spread: React.FC<SpreadProps> = ({ base, quote, mode }) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { currentPrice, previousPrice } = useCurrentPrice();

  const { orderBookStore } = useOrderBookState();
  const orderBook = orderBookStore((s) => s.orderBook);
  const perpsDepth = perpsLiquidityDepthStore((s) => s.liquidityDepth);

  const { spread, spreadPercent } = useMemo(() => {
    if (mode === "perps") {
      if (!perpsDepth) return { spread: null, spreadPercent: null };
      const bidPrices = Object.keys(perpsDepth.bids);
      const askPrices = Object.keys(perpsDepth.asks);
      if (!bidPrices.length || !askPrices.length) return { spread: null, spreadPercent: null };
      const bestBid = bidPrices.reduce((max, p) => (Decimal(p).gt(max) ? p : max), bidPrices[0]);
      const bestAsk = askPrices.reduce((min, p) => (Decimal(p).lt(min) ? p : min), askPrices[0]);
      const mid = Decimal(bestBid).plus(bestAsk).div(2);
      const spreadVal = Decimal(bestAsk).minus(bestBid);
      const spreadPct = mid.gt(0) ? spreadVal.div(mid).times(100) : Decimal(0);
      return { spread: spreadVal, spreadPercent: spreadPct };
    }

    if (!orderBook?.bestAskPrice || !orderBook?.bestBidPrice)
      return { spread: null, spreadPercent: null };
    const mid = Decimal(orderBook.bestBidPrice).plus(orderBook.bestAskPrice).div(2);
    const spreadVal = Decimal(orderBook.bestAskPrice).minus(orderBook.bestBidPrice);
    const spreadPct = mid.gt(0) ? spreadVal.div(mid).times(100) : Decimal(0);
    return { spread: spreadVal, spreadPercent: spreadPct };
  }, [mode, orderBook, perpsDepth]);

  const spreadDisplay = useMemo(() => {
    if (!spread || !spreadPercent) return "n/a";
    const spreadValue =
      mode === "perps"
        ? spread.toFixed()
        : spread.mul(Decimal(10).pow(base.decimals - quote.decimals)).toFixed();
    return `${formatNumber(+spreadValue, formatNumberOptions)} (${formatNumber(spreadPercent.toFixed(), formatNumberOptions)}%)`;
  }, [spread, spreadPercent, mode, base.decimals, quote.decimals, formatNumberOptions]);

  const midPriceDisplay = currentPrice ?? null;

  return (
    <div className="hidden lg:flex w-full py-1 items-center justify-between relative order-2 px-4">
      <p
        className={twMerge(
          "diatype-m-bold relative z-20",
          currentPrice && previousPrice
            ? Decimal(previousPrice).lte(currentPrice)
              ? "text-utility-success-600"
              : "text-utility-error-600"
            : "text-ink-secondary-700",
        )}
      >
        {midPriceDisplay ? <FormattedNumber number={midPriceDisplay} as="span" /> : "-"}
      </p>
      <div className="flex flex-col items-end text-ink-tertiary-500 relative z-20">
        <p className="diatype-xxs-medium">{m["dex.protrade.spread"]()}</p>
        <p className="diatype-xxs-medium">{spreadDisplay}</p>
      </div>
      <span className="bg-surface-tertiary-rice w-[calc(100%+2rem)] absolute -left-4 top-0 h-full z-10" />
    </div>
  );
};

function perpsLiquidityDepthMapper(
  data: {
    bids: Record<string, { size: string; notional: string }>;
    asks: Record<string, { size: string; notional: string }>;
  },
  bucketRecords: number,
  displayMode: "base" | "quote",
) {
  function mapSide(
    entries: Record<string, { size: string; notional: string }>,
    direction: "bid" | "ask",
  ) {
    const sorted = Object.entries(entries)
      .sort(([a], [b]) =>
        direction === "bid" ? (Decimal(a).gt(b) ? -1 : 1) : Decimal(a).gt(b) ? 1 : -1,
      )
      .slice(0, bucketRecords);

    let total = "0";
    let highestSize = "0";
    const records: { price: string; size: string; total: string }[] = [];

    for (const [price, { size, notional }] of sorted) {
      const value = displayMode === "quote" ? notional : size;
      total = Decimal(total).plus(value).toFixed();
      records.push({ price, size: value, total });
      if (Decimal(value).gt(highestSize)) highestSize = value;
    }

    return { records, highestSize };
  }

  return {
    bids: mapSide(data.bids, "bid"),
    asks: mapSide(data.asks, "ask"),
  };
}
