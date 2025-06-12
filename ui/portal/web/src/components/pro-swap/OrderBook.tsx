import { twMerge } from "@left-curve/applets-kit";
import type React from "react";
import { mockOrderBookData, type OrderBookRow } from "~/mock";

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
    <div className="relative flex-1 diatype-sm-medium text-gray-700 grid grid-cols-3">
      <div
        className={twMerge("absolute top-0 bottom-0 opacity-40 z-0", depthBarClass)}
        style={{ width: `${depthBarWidthPercent}%` }}
      />
      <div
        className={twMerge(
          "z-10 text-left",
          type === "bid" ? "text-green-700" : "text-red-bean-700",
        )}
      >
        {price.toFixed(1)}
      </div>
      <div className="z-10 text-end">{amount.toFixed(4)}</div>
      <div className="z-10 text-end">{total.toFixed(2)}</div>
    </div>
  );
};

export const OrderBook: React.FC = () => {
  const { bids, asks } = mockOrderBookData;
  const maxCumulativeAsk = asks.length > 0 ? asks[asks.length - 1].cumulativeTotal : 0;
  const maxCumulativeBid = bids.length > 0 ? bids[bids.length - 1].cumulativeTotal : 0;
  const groupedAsks = groupOrdersByPrice(mockOrderBookData.asks).slice(0, 11);
  const groupedBids = groupOrdersByPrice(mockOrderBookData.bids).slice(0, 11);

  return (
    <div className="flex gap-2 flex-col items-center justify-center ">
      <div className="diatype-xs-medium text-tertiary-500 w-full grid grid-cols-3 ">
        <p>Price</p>
        <p className="text-end">Size (ETH)</p>
        <p className="text-end">Total (ETH)</p>
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
