import { Select, Spinner, useApp, useMediaQuery } from "@left-curve/applets-kit";
import { useEffect, useState } from "react";
import { useRouter } from "@tanstack/react-router";

import { Direction } from "@left-curve/dango/types";
import { calculateTradeSize, Decimal, formatNumber, parseUnits } from "@left-curve/dango/utils";

import { IconLink, ResizerContainer, Tabs, twMerge, formatDate } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { useProTradeState } from "@left-curve/store";

type OrderBookOverviewProps = {
  state: ReturnType<typeof useProTradeState>;
};

export const OrderBookOverview: React.FC<OrderBookOverviewProps> = ({ state }) => {
  const [activeTab, setActiveTab] = useState<"order book" | "trades" | "graph">("graph");

  const { isLg } = useMediaQuery();

  useEffect(() => {
    setActiveTab(isLg ? "trades" : "graph");
  }, [isLg]);

  return (
    <ResizerContainer
      layoutId="order-book-section"
      className="z-10 relative p-4 shadow-account-card bg-surface-secondary-rice flex flex-col gap-2 w-full xl:[width:clamp(279px,20vw,330px)] min-h-[27.25rem] lg:min-h-[37.9rem] h-full"
    >
      <Tabs
        color="line-red"
        layoutId="tabs-order-history"
        selectedTab={activeTab}
        keys={isLg ? ["trades", "order book"] : ["graph", "trades", "order book"]}
        fullWidth
        onTabChange={(tab) => setActiveTab(tab as "order book" | "trades")}
        classNames={{ button: "exposure-xs-italic" }}
      />
      <div
        id="chart-container-mobile"
        className={twMerge("h-full w-full", { hidden: activeTab !== "graph" })}
      />
      {(activeTab === "trades" || activeTab === "order book") && (
        <div className="relative w-full h-full">
          {activeTab === "order book" && <OrderBook state={state} />}
          {activeTab === "trades" && <LiveTrades state={state} />}
        </div>
      )}
    </ResizerContainer>
  );
};

type OrderBookRowProps = {
  price: string;
  size: string;
  total: string;
  type: "bid" | "ask";
};

const OrderRow: React.FC<OrderBookRowProps> = (props) => {
  const { price, size, total, type } = props;
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const depthBarWidthPercent = (Number(size) / Number(total)) * 100;

  const formattedSize = formatNumber(size, {
    ...formatNumberOptions,
    maxSignificantDigits: 10,
    maxFractionDigits: 5,
  }).slice(0, 7);

  const isAmountTooSmall = Decimal(size).lt(0.00001);

  const depthBarClass =
    type === "bid"
      ? "bg-status-success lg:-left-4"
      : "bg-status-fail -right-0 lg:-left-4 lg:right-auto";

  return (
    <div className="relative flex-1 diatype-xs-medium text-secondary-700 grid grid-cols-2 lg:grid-cols-3">
      <div
        className={twMerge("absolute top-0 bottom-0 opacity-20 z-0", depthBarClass)}
        style={{ width: `${depthBarWidthPercent}%` }}
      />
      <div
        className={twMerge(
          "z-10",
          type === "bid"
            ? "text-status-success text-left"
            : "text-status-fail order-2 lg:order-none text-end lg:text-left",
        )}
      >
        {formatNumber(price, {
          ...formatNumberOptions,
          minSignificantDigits: 8,
          maxSignificantDigits: 8,
        })}
      </div>
      <div className="z-10 justify-center text-center hidden lg:flex gap-1">
        {isAmountTooSmall ? (
          <>
            <span>{"<"}</span>
            {"0.00001"}
          </>
        ) : (
          formattedSize
        )}
      </div>
      <div
        className={twMerge(
          "z-10",
          type === "bid" ? "text-end" : "order-1 lg:order-none lg:text-end",
        )}
      >
        {formatNumber(total, {
          ...formatNumberOptions,
          minSignificantDigits: 8,
          maxSignificantDigits: 8,
        })}
      </div>
    </div>
  );
};

const OrderBook: React.FC<OrderBookOverviewProps> = ({ state }) => {
  const { isLg } = useMediaQuery();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const {
    baseCoin,
    quoteCoin,
    liquidityDepth,
    previousPrice,
    orderBookState,
    pair,
    bucketSize,
    setBucketSize,
    bucketSizeCoin,
    setBucketSizeCoin,
  } = state;

  if (!liquidityDepth) return <Spinner fullContainer size="md" color="pink" />;

  const currentPrice = parseUnits(
    orderBookState?.midPrice as string,
    baseCoin.decimals - quoteCoin.decimals,
  );

  const { bids, asks } = liquidityDepth;
  const numberOfOrders = isLg ? 11 : 16;

  return (
    <div className="flex gap-2 flex-col items-center justify-center ">
      <div className="flex items-center justify-between w-full">
        <Select value={bucketSize} onChange={(key) => setBucketSize(key)} variant="plain">
          {pair.params.bucketSizes.map((size) => {
            return (
              <Select.Item key={`bucket-${size}`} value={size}>
                {size}
              </Select.Item>
            );
          })}
        </Select>
        <Select
          value={bucketSizeCoin}
          onChange={(key) => setBucketSizeCoin(key)}
          variant="plain"
          classNames={{ listboxWrapper: "right-0 left-auto" }}
        >
          <Select.Item value={baseCoin.symbol}>{baseCoin.symbol}</Select.Item>
          <Select.Item value={quoteCoin.symbol}>{quoteCoin.symbol}</Select.Item>
        </Select>
      </div>
      <div className="diatype-xs-medium text-tertiary-500 w-full grid grid-cols-4 lg:grid-cols-3 gap-2">
        <p className="order-2 lg:order-none text-end lg:text-start">
          {m["dex.protrade.history.price"]()}
        </p>
        <p className="text-center hidden lg:block">
          {m["dex.protrade.history.size"]({ symbol: bucketSizeCoin })}
        </p>
        <p className="lg:text-end order-1 lg:order-none">
          {m["dex.protrade.history.total"]({ symbol: bucketSizeCoin })}
        </p>
        <p className="order-3 lg:hidden">{m["dex.protrade.history.price"]()}</p>
        <p className="order-4 text-end lg:order-none lg:hidden">
          {m["dex.protrade.history.total"]({ symbol: bucketSizeCoin })}
        </p>
      </div>
      <div className="flex gap-2 lg:flex-col items-start justify-center w-full tabular-nums lining-nums">
        <div className="asks-container flex flex-1 flex-col w-full gap-1">
          {asks.records.slice(0, numberOfOrders).map((ask) => (
            <OrderRow key={`ask-${ask.price}`} type="ask" {...ask} />
          ))}
        </div>

        <div className="hidden lg:flex  w-full p-2 items-center justify-center relative">
          <p
            className={twMerge(
              "diatype-xs-bold relative z-20",
              Decimal(previousPrice).lte(currentPrice) ? "text-status-fail" : "text-status-success",
            )}
          >
            {formatNumber(currentPrice || "0", {
              ...formatNumberOptions,
              minSignificantDigits: 8,
              maxSignificantDigits: 8,
            })}
          </p>
          <span className="bg-surface-tertiary-rice w-[calc(100%+2rem)] absolute -left-4 top-0 h-full z-10" />
        </div>

        <div className="bid-container flex flex-1 flex-col w-full gap-1">
          {bids.records.slice(0, numberOfOrders).map((bid) => (
            <OrderRow key={`bid-${bid.price}`} type="bid" {...bid} />
          ))}
        </div>
      </div>
    </div>
  );
};

const LiveTrades: React.FC<OrderBookOverviewProps> = ({ state }) => {
  const { navigate } = useRouter();
  const { settings } = useApp();
  const { formatNumberOptions, timeFormat } = settings;
  const { trades, baseCoin, quoteCoin } = state;

  return (
    <div className="flex gap-2 flex-col items-center justify-start max-h-[23.1375rem] lg:max-h-[53.55vh] overflow-hidden">
      <div className="diatype-xs-medium text-tertiary-500 w-full grid grid-cols-3">
        <p>{m["dex.protrade.history.price"]()}</p>
        <p className="text-center">{m["dex.protrade.history.size"]({ symbol: baseCoin.symbol })}</p>
        <p className="text-end">{m["dex.protrade.history.time"]()}</p>
      </div>
      <div className="relative flex-1 w-full flex flex-col gap-1 items-center tabular-nums lining-nums">
        {trades.map((trade, index) => {
          const size = calculateTradeSize(trade, baseCoin.decimals).toFixed();

          const formattedSize = formatNumber(size, {
            ...formatNumberOptions,
            maxSignificantDigits: 10,
            maxFractionDigits: 5,
          }).slice(0, 7);

          const isAmountTooSmall = Decimal(size).lt(0.00001);
          return (
            <div
              key={`${trade.addr}-${trade.createdAt}-${index}`}
              onClick={() => navigate({ to: `/block/${trade.blockHeight}` })}
              className={
                "grid grid-cols-3 diatype-xs-medium text-secondary-700 w-full cursor-pointer group relative"
              }
            >
              <p
                className={twMerge(
                  "z-10",
                  trade.direction === Direction.Buy ? "text-status-success" : "text-status-fail",
                )}
              >
                {formatNumber(
                  Decimal(trade.clearingPrice)
                    .times(Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals))
                    .toFixed(),
                  { ...formatNumberOptions, minSignificantDigits: 8, maxSignificantDigits: 8 },
                ).slice(0, 10)}
              </p>
              <p className="text-center z-10 flex gap-1 justify-center">
                {isAmountTooSmall ? (
                  <>
                    <span>{"<"}</span>
                    {"0.00001"}
                  </>
                ) : (
                  formattedSize
                )}
              </p>

              <div className="flex flex-nowrap whitespace-nowrap gap-1 items-center justify-end z-10">
                <p>{formatDate(trade.createdAt, timeFormat.replace("mm", "mm:ss"))}</p>
                <IconLink className="w-3 h-3" />
              </div>
              <span className="group-hover:bg-surface-tertiary-rice h-[calc(100%+0.5rem)] w-[calc(100%+2rem)] absolute top-[-0.25rem] -left-4 z-0" />
            </div>
          );
        })}
      </div>
    </div>
  );
};
