import { FormattedNumber, Select, Spinner, useApp, useMediaQuery } from "@left-curve/applets-kit";
import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "@tanstack/react-router";

import {
  useLivePerpsTrades,
  useCurrentPrice,
  useTradePairCoins,
  useAppConfig,
  useStorage,
  usePerpsLiquidityDepth,
  usePerpsOrdersByUser,
} from "@left-curve/store";
import { useProTrade } from "./ProTrade";
import { bucketSizeToFractionDigits, Decimal, formatNumber } from "@left-curve/utils";

import { IconLink, ResizerContainer, Tabs, twMerge, formatDate } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { AnyCoin } from "@left-curve/store/types";
import type { Controllers } from "@left-curve/applets-kit";
import type { PerpsLiquidityDepthResponse } from "@left-curve/types";

type OrderBookOverviewProps = {
  controllers: Controllers;
};

export const OrderBookOverview: React.FC<OrderBookOverviewProps> = ({ controllers }) => {
  const [activeTab, setActiveTab] = useState<"order book" | "trades" | "graph">("graph");

  const { isLg, is3XlTall } = useMediaQuery();

  const { pairId, perpsPairId, accountAddress } = useProTrade();
  const { data: appConfig } = useAppConfig();

  const { baseCoin, quoteCoin } = useTradePairCoins({ pairId });

  const pairInfo = appConfig.perpsPairs[perpsPairId];

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
              baseCoin={baseCoin}
              quoteCoin={quoteCoin}
              bucketSizes={bucketSizes}
              bucketSize={bucketSize}
              setBucketSize={setBucketSize}
              bucketRecords={bucketRecords}
              perpsPairId={perpsPairId}
              accountAddress={accountAddress}
              controllers={controllers}
            />
          )}
          {activeTab === "trades" && <LiveTrades baseCoin={baseCoin} perpsPairId={perpsPairId} />}
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
          <LiveTrades baseCoin={baseCoin} perpsPairId={perpsPairId} />
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
  userOrderPrices: Set<string>;
};

const OrderRow: React.FC<OrderBookRowProps> = (props) => {
  const {
    price,
    size,
    total,
    type,
    max,
    priceFractionDigits,
    onSelectPrice,
    flashKey,
    userOrderPrices,
  } = props;
  const depthBarWidthPercent = Decimal(size).div(max).times(100).toFixed();
  const hasUserOrder = userOrderPrices.has(Decimal(price).toFixed(priceFractionDigits));

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

type OrderBookProps = {
  baseCoin: AnyCoin & { amount: string };
  quoteCoin: AnyCoin & { amount: string };
  bucketSizes: string[];
  bucketSize: string;
  setBucketSize: (size: string) => void;
  bucketRecords: number;
  perpsPairId: string;
  accountAddress?: string;
  controllers: Controllers;
};

const OrderBook: React.FC<OrderBookProps> = ({
  baseCoin,
  quoteCoin,
  bucketSizes,
  bucketSize,
  setBucketSize,
  bucketRecords,
  perpsPairId,
  accountAddress,
  controllers,
}) => {
  const [perpsDisplayModeRaw, setPerpsDisplayMode] = useStorage<"base" | "quote">(
    "perps-order-book-display-mode",
    { initialValue: "base" },
  );
  const perpsDisplayMode = perpsDisplayModeRaw === "quote" ? "quote" : "base";

  const bucketSizeSymbol = perpsDisplayMode === "quote" ? "USD" : baseCoin.symbol;

  const priceFractionDigits = useMemo(() => bucketSizeToFractionDigits(bucketSize), [bucketSize]);

  return (
    <div className="flex gap-2 flex-col items-center justify-center h-full">
      <div className="flex items-center justify-between w-full px-4">
        <Select value={bucketSize} onChange={(key) => setBucketSize(key)} variant="plain">
          {bucketSizes.map((size: string) => (
            <Select.Item key={`bucket-${size}`} value={size}>
              {size}
            </Select.Item>
          ))}
        </Select>
        <Select
          value={perpsDisplayMode}
          onChange={(key) => setPerpsDisplayMode(key as "base" | "quote")}
          variant="plain"
          classNames={{ listboxWrapper: "right-0 left-auto" }}
        >
          <Select.Item value="base">{baseCoin.symbol}</Select.Item>
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
        perpsPairId={perpsPairId}
        accountAddress={accountAddress}
        bucketSize={bucketSize}
        bucketRecords={bucketRecords}
        base={baseCoin}
        quote={quoteCoin}
        onSelectPrice={(price) => controllers.setValue("price", price)}
        displayMode={perpsDisplayMode}
        priceFractionDigits={priceFractionDigits}
      />
    </div>
  );
};

type LiveTradesProps = {
  baseCoin: AnyCoin & { amount: string };
  perpsPairId: string;
};

const LiveTrades: React.FC<LiveTradesProps> = ({ baseCoin, perpsPairId }) => {
  const { navigate } = useRouter();
  const { settings } = useApp();
  const { is3XlTall } = useMediaQuery();
  const { timeFormat } = settings;

  const livePerps = useLivePerpsTrades((s) => s.trades, { perpsPairId });
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

type LiquidityDepthProps = {
  perpsPairId: string;
  accountAddress?: string;
  bucketSize: string;
  bucketRecords: number;
  base: AnyCoin;
  quote: AnyCoin;
  onSelectPrice: (price: string) => void;
  displayMode: "base" | "quote";
  priceFractionDigits: number;
};

const LiquidityDepth: React.FC<LiquidityDepthProps> = ({
  perpsPairId,
  accountAddress,
  bucketSize,
  bucketRecords,
  onSelectPrice,
  displayMode,
  priceFractionDigits,
}) => {
  const { isLg } = useMediaQuery();
  const perpsDepthData = usePerpsLiquidityDepth((s) => s.liquidityDepth, {
    perpsPairId,
    bucketSize,
  });

  const perpsOrdersData = usePerpsOrdersByUser((s) => s.orders, { accountAddress });

  const userOrderPrices = useMemo(() => {
    const prices = new Set<string>();
    if (perpsOrdersData) {
      for (const o of Object.values(perpsOrdersData)) {
        if (o.pairId !== perpsPairId) continue;
        prices.add(Decimal(o.limitPrice).toFixed(priceFractionDigits));
      }
    }
    return prices;
  }, [perpsOrdersData, priceFractionDigits, perpsPairId]);

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
            priceFractionDigits={priceFractionDigits}
            onSelectPrice={onSelectPrice}
            flashKey={flashKeys.get(ask.price)}
            userOrderPrices={userOrderPrices}
          />
        ))}
      </div>

      <Spread perpsPairId={perpsPairId} perpsDepth={perpsDepthData} />

      <div className="bid-container flex flex-1 flex-col w-full gap-[2px] order-1 lg:order-3">
        {[...bids.records].map((bid, i) => (
          <OrderRow
            key={`bid-${bid.price}-${i}`}
            type="bid"
            {...bid}
            max={max}
            priceFractionDigits={priceFractionDigits}
            onSelectPrice={onSelectPrice}
            flashKey={flashKeys.get(bid.price)}
            userOrderPrices={userOrderPrices}
          />
        ))}
      </div>
    </div>
  );
};

const Spread: React.FC<{
  perpsPairId: string;
  perpsDepth: PerpsLiquidityDepthResponse | null;
}> = ({ perpsPairId, perpsDepth }) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { currentPrice, previousPrice } = useCurrentPrice({ perpsPairId });

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
