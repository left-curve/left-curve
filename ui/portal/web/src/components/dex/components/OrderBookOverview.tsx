import { FormattedNumber, Select, Spinner, useMediaQuery } from "@left-curve/applets-kit";
import { memo, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "@tanstack/react-router";

import {
  useLivePerpsTrades,
  useCurrentPrice,
  useAppConfig,
  useStorage,
  usePerpsLiquidityDepth,
  usePerpsOrdersByUser,
} from "@left-curve/store";
import { useProTrade } from "./ProTrade";
import { bucketSizeToFractionDigits, Decimal, formatNumber, shallowEqual } from "@left-curve/utils";

import { IconLink, ResizerContainer, Tabs, twMerge, formatDate } from "@left-curve/applets-kit";
import { useApp } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { BaseCoin } from "@left-curve/store/types";
import type { PerpsLiquidityDepthResponse } from "@left-curve/types";

type OrderBookOverviewProps = {
  onSelectPrice: (price: string) => void;
};

const ORDER_BOOK_VISUAL_UPDATE_MS = 500;

const OrderBookOverviewComponent: React.FC<OrderBookOverviewProps> = ({ onSelectPrice }) => {
  const [activeTab, setActiveTab] = useState<"order book" | "trades" | "graph">("graph");

  const { isLg, is3XlTall } = useMediaQuery();

  const { pair, accountAddress } = useProTrade();
  const { data: appConfig } = useAppConfig();
  const pairId = pair.id;

  const base = pair.base;
  const quote = pair.quote;

  const pairInfo = appConfig.perpsPairs[pairId];

  const bucketSizes: string[] = pairInfo.bucketSizes;

  const [bucketSize, setBucketSize] = useState(bucketSizes[0]);

  useEffect(() => {
    setBucketSize(bucketSizes[0]);
  }, [bucketSizes]);

  useEffect(() => {
    if (is3XlTall) {
      setActiveTab("order book");
    } else {
      setActiveTab(isLg ? "order book" : "graph");
    }
  }, [isLg, is3XlTall]);

  const tabsKeys = useMemo(() => {
    if (is3XlTall) {
      return ["order book"];
    }
    return isLg ? ["order book", "trades"] : ["graph", "order book", "trades"];
  }, [isLg, is3XlTall]);

  const bucketRecords = isLg ? 10 : 16;

  return (
    <ResizerContainer
      layoutId="order-book-section"
      className={twMerge(
        "overflow-hidden z-10 relative p-0 shadow-account-card bg-surface-primary-rice flex flex-col gap-2 w-full xl:[width:clamp(279px,20vw,330px)] min-h-[27.25rem] lg:h-[38.65625rem] 3xl:min-h-[40rem] h-full",
        { "3xl:min-h-[51.6875rem] 4xl:min-h-[61.6875rem]": is3XlTall },
      )}
    >
      <Tabs
        color="line-red"
        layoutId="tabs-order-history"
        selectedTab={activeTab}
        keys={tabsKeys}
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
          {activeTab === "order book" && (
            <OrderBook
              base={base}
              quote={quote}
              bucketSizes={bucketSizes}
              bucketSize={bucketSize}
              setBucketSize={setBucketSize}
              bucketRecords={bucketRecords}
              pairId={pairId}
              accountAddress={accountAddress}
              onSelectPrice={onSelectPrice}
            />
          )}
          {activeTab === "trades" && <LiveTrades base={base} pairId={pairId} />}
        </>
      )}
      {is3XlTall && (
        <>
          <Tabs
            color="line-red"
            layoutId="tabs-order-history-2"
            selectedTab={"trades"}
            keys={["trades"]}
            fullWidth
            classNames={{ button: "exposure-xs-italic", base: "px-4 pt-4" }}
          />
          <LiveTrades base={base} pairId={pairId} />
        </>
      )}
    </ResizerContainer>
  );
};

export const OrderBookOverview = memo(OrderBookOverviewComponent);
OrderBookOverview.displayName = "OrderBookOverview";

type OrderBookRowProps = {
  price: string;
  size: string;
  total: string;
  max: string;
  type: "bid" | "ask";
  priceFractionDigits: number;
  onSelectPrice: (price: string) => void;
  flashKey?: number;
  hasUserOrder: boolean;
};

const OrderRowComponent: React.FC<OrderBookRowProps> = (props) => {
  const {
    price,
    size,
    total,
    type,
    max,
    priceFractionDigits,
    onSelectPrice,
    flashKey,
    hasUserOrder,
  } = props;
  const depthBarWidthPercent = Decimal(size).div(max).times(100).toFixed();

  const depthBarClass =
    type === "bid"
      ? "bg-utility-success-500 lg:right-auto right-0"
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
        {hasUserOrder && (
          <span
            aria-hidden
            className={twMerge(
              "absolute left-1 top-1/2 -translate-y-1/2 w-1.5 h-1.5 rounded-full z-10",
              type === "bid" ? "bg-utility-success-500" : "bg-utility-error-500",
            )}
          />
        )}
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

const OrderRow = memo(OrderRowComponent);
OrderRow.displayName = "OrderRow";

type OrderBookProps = {
  base: BaseCoin;
  quote: BaseCoin;
  bucketSizes: string[];
  bucketSize: string;
  setBucketSize: (size: string) => void;
  bucketRecords: number;
  pairId: string;
  accountAddress?: string;
  onSelectPrice: (price: string) => void;
};

const OrderBook: React.FC<OrderBookProps> = ({
  base,
  quote,
  bucketSizes,
  bucketSize,
  setBucketSize,
  bucketRecords,
  pairId,
  accountAddress,
  onSelectPrice,
}) => {
  const [perpsDisplayModeRaw, setPerpsDisplayMode] = useStorage<"base" | "quote">(
    "perps-order-book-display-mode",
    { initialValue: "base" },
  );
  const perpsDisplayMode = perpsDisplayModeRaw === "quote" ? "quote" : "base";

  const bucketSizeSymbol = perpsDisplayMode === "quote" ? "USD" : base.symbol;

  const priceFractionDigits = useMemo(() => bucketSizeToFractionDigits(bucketSize), [bucketSize]);
  const handleDisplayModeChange = useCallback(
    (key: string) => setPerpsDisplayMode(key as "base" | "quote"),
    [setPerpsDisplayMode],
  );

  return (
    <div className="flex gap-2 flex-col items-center justify-center h-full">
      <div className="flex items-center justify-between w-full px-4">
        <Select value={bucketSize} onChange={setBucketSize} variant="plain">
          {bucketSizes.map((size: string) => (
            <Select.Item key={`bucket-${size}`} value={size}>
              {size}
            </Select.Item>
          ))}
        </Select>
        <Select
          value={perpsDisplayMode}
          onChange={handleDisplayModeChange}
          variant="plain"
          classNames={{ listboxWrapper: "right-0 left-auto" }}
        >
          <Select.Item value="base">{base.symbol}</Select.Item>
          <Select.Item value="quote">USD</Select.Item>
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
        accountAddress={accountAddress}
        bucketSize={bucketSize}
        bucketRecords={bucketRecords}
        base={base}
        quote={quote}
        onSelectPrice={onSelectPrice}
        displayMode={perpsDisplayMode}
        priceFractionDigits={priceFractionDigits}
      />
    </div>
  );
};

type LiveTradesProps = {
  base: BaseCoin;
  pairId: string;
};

const LiveTrades: React.FC<LiveTradesProps> = ({ base, pairId }) => {
  const { navigate } = useRouter();
  const timeFormat = useApp((state) => state.settings.timeFormat);
  const { is3XlTall } = useMediaQuery();

  const livePerps = useLivePerpsTrades((s) => s.trades, { pairId });
  const perpsTrades = useDeferredValue(livePerps);

  return (
    <div
      className={twMerge(
        "flex gap-2 flex-col items-center justify-start lg:max-h-[38.75rem] overflow-y-scroll scrollbar-none overflow-x-hidden relative px-4",
        is3XlTall && "max-h-[15rem] min-h-[15rem] 4xl:max-h-[20rem] 4xl:min-h-[20rem]",
      )}
    >
      <div className="diatype-xs-medium text-ink-tertiary-500 w-full grid grid-cols-3 sticky top-0 bg-surface-primary-rice z-20">
        <p>{m["dex.protrade.history.price"]()}</p>
        <p className="text-center">{m["dex.protrade.history.size"]({ symbol: base.symbol })}</p>
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

type LiquidityDepthProps = {
  pairId: string;
  accountAddress?: string;
  bucketSize: string;
  bucketRecords: number;
  base: BaseCoin;
  quote: BaseCoin;
  onSelectPrice: (price: string) => void;
  displayMode: "base" | "quote";
  priceFractionDigits: number;
};

const LiquidityDepth: React.FC<LiquidityDepthProps> = ({
  pairId,
  accountAddress,
  bucketSize,
  bucketRecords,
  onSelectPrice,
  displayMode,
  priceFractionDigits,
}) => {
  const { isLg } = useMediaQuery();
  const { liquidityDepth: perpsDepthData, error: perpsDepthError } = usePerpsLiquidityDepth(
    (s) => ({ liquidityDepth: s.liquidityDepth, error: s.error }),
    {
      pairId,
      bucketSize,
      notifyIntervalMs: ORDER_BOOK_VISUAL_UPDATE_MS,
    },
    shallowEqual,
  );

  const userOrderPrices = usePerpsOrdersByUser(
    (s) => {
      if (!s.orders) return [];
      const prices = new Set<string>();
      for (const order of Object.values(s.orders)) {
        if (order.pairId !== pairId) continue;
        prices.add(Decimal(order.limitPrice).toFixed(priceFractionDigits));
      }
      return [...prices].sort();
    },
    { accountAddress },
    shallowEqual,
  );

  const userOrderPriceSet = useMemo(() => new Set(userOrderPrices), [userOrderPrices]);

  const liquidityDepth = useMemo(() => {
    if (!perpsDepthData) return null;
    return perpsLiquidityDepthMapper(perpsDepthData, bucketRecords, displayMode);
  }, [perpsDepthData, bucketRecords, displayMode]);

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

  const asksOrdered = useMemo(() => {
    if (!liquidityDepth) return [];
    return isLg ? [...liquidityDepth.asks.records].reverse() : liquidityDepth.asks.records;
  }, [liquidityDepth, isLg]);

  if (perpsDepthError) {
    return (
      <div className="flex h-full min-h-[12rem] w-full items-center justify-center p-4 text-center diatype-xs-medium text-utility-error-600">
        Order book unavailable
      </div>
    );
  }

  if (!liquidityDepth) return <Spinner fullContainer size="md" color="pink" />;

  const { bids, asks } = liquidityDepth;

  const max = Decimal.max(bids.highestSize, asks.highestSize).toFixed();

  return (
    <div className="flex-1 h-full flex gap-2 lg:flex-col items-start justify-center w-full">
      <div className="asks-container flex flex-1 flex-col w-full gap-[2px] order-2 lg:order-1 lg:justify-end">
        {asksOrdered.map((ask) => (
          <OrderRow
            key={`ask-${ask.price}`}
            type="ask"
            {...ask}
            max={max}
            priceFractionDigits={priceFractionDigits}
            onSelectPrice={onSelectPrice}
            flashKey={flashKeys.get(ask.price)}
            hasUserOrder={userOrderPriceSet.has(Decimal(ask.price).toFixed(priceFractionDigits))}
          />
        ))}
      </div>

      <Spread pairId={pairId} perpsDepth={perpsDepthData} />

      <div className="bid-container flex flex-1 flex-col w-full gap-[2px] order-1 lg:order-3">
        {bids.records.map((bid) => (
          <OrderRow
            key={`bid-${bid.price}`}
            type="bid"
            {...bid}
            max={max}
            priceFractionDigits={priceFractionDigits}
            onSelectPrice={onSelectPrice}
            flashKey={flashKeys.get(bid.price)}
            hasUserOrder={userOrderPriceSet.has(Decimal(bid.price).toFixed(priceFractionDigits))}
          />
        ))}
      </div>
    </div>
  );
};

const Spread: React.FC<{
  pairId: string;
  perpsDepth: PerpsLiquidityDepthResponse | null;
}> = ({ pairId, perpsDepth }) => {
  const formatNumberOptions = useApp((state) => state.settings.formatNumberOptions);

  const { currentPrice, previousPrice } = useCurrentPrice({ pairId });

  const { spread, spreadPercent } = useMemo(() => {
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
  }, [perpsDepth]);

  const spreadDisplay = useMemo(() => {
    if (!spread || !spreadPercent) return "n/a";
    return `${formatNumber(+spread.toFixed(), formatNumberOptions)} (${formatNumber(spreadPercent.toFixed(), formatNumberOptions)}%)`;
  }, [spread, spreadPercent, formatNumberOptions]);

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
