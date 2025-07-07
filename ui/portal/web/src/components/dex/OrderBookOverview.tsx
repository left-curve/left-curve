import { useMediaQuery } from "@left-curve/applets-kit";
import { useEffect, useState } from "react";

import { IconLink, ResizerContainer, Tabs, twMerge } from "@left-curve/applets-kit";
import { TradingViewChart } from "./TradingViewChart";

import type React from "react";

import { mockTrades } from "~/mock";
import { type OrderBookRow, mockOrderBookData } from "~/mock";

export const OrderBookOverview: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"order book" | "trades" | "graph">("graph");

  const { isLg } = useMediaQuery();

  useEffect(() => {
    setActiveTab(isLg ? "order book" : "graph");
  }, [isLg]);

  return (
    <ResizerContainer
      layoutId="order-book-section"
      className="p-4 shadow-card-shadow bg-rice-25 flex flex-col gap-2 w-full xl:[width:clamp(279px,20vw,330px)] min-h-[27.25rem] lg:min-h-[37.9rem]"
    >
      <Tabs
        color="line-red"
        layoutId="tabs-order-history"
        selectedTab={activeTab}
        keys={isLg ? ["order book", "trades"] : ["graph", "order book", "trades"]}
        fullWidth
        onTabChange={(tab) => setActiveTab(tab as "order book" | "trades")}
        classNames={{ button: "exposure-xs-italic pt-0" }}
      />
      {activeTab === "graph" && <TradingViewChart />}
      {activeTab === "order book" && <OrderBook />}
      {activeTab === "trades" && <LiveTrades />}
    </ResizerContainer>
  );
};

function groupOrdersByPrice(orders: { price: number; amount: number }[]) {
  const groupedMap = new Map<number, number>();

  for (const order of orders) {
    groupedMap.set(order.price, (groupedMap.get(order.price) || 0) + order.amount);
  }

  const groupedArray: OrderBookRow[] = [];
  let cumulative = 0;

  const sorted = [...groupedMap.entries()].sort((a, b) => b[0] - a[0]);

  for (const [price, amount] of sorted) {
    const total = price * amount;
    cumulative += total;
    groupedArray.push({ price, amount, total, cumulativeTotal: cumulative });
  }

  return groupedArray;
}

const OrderRow: React.FC<
  OrderBookRow & {
    type: "bid" | "ask";
    maxCumulativeTotal: number;
  }
> = ({ price, amount, total, cumulativeTotal, maxCumulativeTotal, type }) => {
  const depthBarWidthPercent =
    maxCumulativeTotal > 0 ? (cumulativeTotal / maxCumulativeTotal) * 100 : 0;

  const depthBarClass =
    type === "bid" ? "bg-green-300 lg:-left-4" : "bg-red-300 -right-0 lg:-left-4 lg:right-auto";

  return (
    <div className="relative flex-1 diatype-xs-medium text-gray-700 grid grid-cols-2 lg:grid-cols-3">
      <div
        className={twMerge("absolute top-0 bottom-0 opacity-40 z-0", depthBarClass)}
        style={{ width: `${depthBarWidthPercent}%` }}
      />
      <div
        className={twMerge(
          "z-10",
          type === "bid"
            ? "text-green-700 text-left"
            : "text-red-bean-700 order-2 lg:order-none text-end lg:text-left",
        )}
      >
        {price.toFixed(1)}
      </div>
      <div className="z-10 text-end hidden lg:block">{amount.toFixed(4)}</div>
      <div
        className={twMerge(
          "z-10",
          type === "bid" ? "text-end" : "order-1 lg:order-none lg:text-end",
        )}
      >
        {total.toFixed(2)}
      </div>
    </div>
  );
};

const OrderBook: React.FC = () => {
  const { isLg } = useMediaQuery();
  const { bids, asks } = mockOrderBookData;
  const maxCumulativeAsk = asks.length > 0 ? asks[asks.length - 1].cumulativeTotal : 0;
  const maxCumulativeBid = bids.length > 0 ? bids[bids.length - 1].cumulativeTotal : 0;
  const numberOfOrders = isLg ? 11 : 16;
  const groupedAsks = groupOrdersByPrice(mockOrderBookData.asks).slice(0, numberOfOrders);
  const groupedBids = groupOrdersByPrice(mockOrderBookData.bids).slice(0, numberOfOrders);

  return (
    <div className="flex gap-2 flex-col items-center justify-center ">
      <div className="diatype-xs-medium text-gray-500 w-full grid grid-cols-4 lg:grid-cols-3 gap-2">
        <p className="order-2 lg:order-none text-end lg:text-start">Price</p>
        <p className="text-end hidden lg:block">Size (ETH)</p>
        <p className="lg:text-end order-1 lg:order-none">Total (ETH)</p>
        <p className="order-3 lg:hidden">Price</p>
        <p className="order-4 text-end lg:order-none lg:hidden">Total (ETH)</p>
      </div>
      <div className="flex gap-2 lg:flex-col items-center justify-center w-full">
        <div className="asks-container flex flex-col w-full gap-1">
          {groupedAsks.slice().map((ask) => (
            <OrderRow
              key={`ask-${ask.price}`}
              {...ask}
              type="ask"
              maxCumulativeTotal={maxCumulativeAsk}
            />
          ))}
        </div>
        {bids.length > 0 && asks.length > 0 && (
          <div className="hidden lg:flex  w-full p-2 items-center justify-center relative">
            <p className="diatype-xs-bold text-status-success relative z-20">
              {bids[bids.length - 1].price.toFixed(2)}
            </p>
            <span className="bg-rice-50 w-[calc(100%+2rem)] absolute -left-4 top-0 h-full z-10" />
          </div>
        )}
        <div className="bid-container flex flex-col w-full gap-1">
          {groupedBids.slice().map((bid) => (
            <OrderRow
              key={`bid-${bid.price}`}
              {...bid}
              type="bid"
              maxCumulativeTotal={maxCumulativeBid}
            />
          ))}
        </div>
      </div>
    </div>
  );
};

const LiveTrades: React.FC = () => {
  const { isLg } = useMediaQuery();
  const numberOfTrades = isLg ? 24 : 16;
  return (
    <div className="flex gap-2 flex-col items-center justify-center ">
      <div className="diatype-xs-medium text-gray-500 w-full grid grid-cols-3 ">
        <p>Price</p>
        <p className="text-end">Size (ETH)</p>
        <p className="text-end">Time</p>
      </div>
      <div className="relative flex-1 w-full flex flex-col gap-1 items-center">
        {mockTrades.slice(0, numberOfTrades).map((trade) => {
          return (
            <div
              key={trade.hash}
              className={
                "grid grid-cols-3 diatype-xs-medium text-gray-700 w-full cursor-pointer group relative"
              }
            >
              <p
                className={twMerge(
                  "z-10",
                  trade.side === "BUY" ? "text-status-success" : "text-status-fail",
                )}
              >
                {trade.price}
              </p>
              <p className="text-end z-10">{trade.size}</p>

              <div className="flex gap-1 items-center justify-end z-10">
                <p>{trade.createdAt}</p>
                <IconLink className="w-3 h-3" />
              </div>
              <span className="group-hover:bg-rice-50 h-[calc(100%+0.5rem)] w-[calc(100%+2rem)] absolute top-[-0.25rem] -left-4 z-0" />
            </div>
          );
        })}
      </div>
    </div>
  );
};
