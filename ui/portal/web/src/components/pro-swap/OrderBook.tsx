import { Tabs, twMerge } from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";

interface OrderBookRow {
  price: number;
  amount: number;
  total: number;
  cumulativeTotal: number;
}

type OrderBookData = {
  bids: OrderBookRow[];
  asks: OrderBookRow[];
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

const mockOrderBookData: OrderBookData = {
  bids: [
    { price: 1899.2, amount: 0.5, total: 949.6, cumulativeTotal: 949.6 },
    { price: 1899.1, amount: 1.2, total: 2278.92, cumulativeTotal: 3228.52 },
    { price: 1898.5, amount: 0.8, total: 1518.8, cumulativeTotal: 4747.32 },
    { price: 1898.3, amount: 2.1, total: 3986.43, cumulativeTotal: 8733.75 },
    { price: 1898.0, amount: 0.3, total: 569.4, cumulativeTotal: 9303.15 },
    { price: 1897.9, amount: 1.5, total: 2846.85, cumulativeTotal: 12150.0 },
    { price: 1897.5, amount: 0.7, total: 1328.25, cumulativeTotal: 13478.25 },
    { price: 1897.2, amount: 1.9, total: 3604.68, cumulativeTotal: 17082.93 },
    { price: 1896.8, amount: 0.4, total: 758.72, cumulativeTotal: 17841.65 },
    { price: 1896.5, amount: 2.5, total: 4741.25, cumulativeTotal: 22582.9 },
    { price: 1896.1, amount: 0.6, total: 1137.66, cumulativeTotal: 23720.56 },
    { price: 1895.9, amount: 1.1, total: 2085.49, cumulativeTotal: 25806.05 },
    { price: 1895.4, amount: 0.9, total: 1705.86, cumulativeTotal: 27511.91 },
    { price: 1895.0, amount: 3.0, total: 5685.0, cumulativeTotal: 33196.91 },
    { price: 1894.7, amount: 0.2, total: 378.94, cumulativeTotal: 33575.85 },
    { price: 1894.3, amount: 1.8, total: 3409.74, cumulativeTotal: 36985.59 },
    { price: 1893.9, amount: 0.75, total: 1420.425, cumulativeTotal: 38406.015 },
    { price: 1893.5, amount: 1.3, total: 2461.55, cumulativeTotal: 40867.565 },
    { price: 1893.1, amount: 0.85, total: 1609.135, cumulativeTotal: 42476.7 },
    { price: 1892.8, amount: 2.2, total: 4164.16, cumulativeTotal: 46640.86 },
    { price: 1892.56, amount: 1.3512, total: 2556.5327, cumulativeTotal: 49197.3927 },
    { price: 1892.21, amount: 0.6788, total: 1284.3785, cumulativeTotal: 50481.7712 },
    { price: 1891.93, amount: 2.4501, total: 4635.5064, cumulativeTotal: 55117.2776 },
    { price: 1891.68, amount: 1.1234, total: 2125.3273, cumulativeTotal: 57242.6049 },
    { price: 1891.45, amount: 0.3333, total: 630.4205, cumulativeTotal: 57873.0254 },
    { price: 1891.03, amount: 1.8876, total: 3570.0013, cumulativeTotal: 61443.0267 },
    { price: 1890.77, amount: 0.9234, total: 1746.1532, cumulativeTotal: 63189.1799 },
    { price: 1890.35, amount: 2.0567, total: 3887.9799, cumulativeTotal: 67077.1598 },
    { price: 1890.01, amount: 1.5552, total: 2939.3806, cumulativeTotal: 70016.5404 },
    { price: 1889.72, amount: 0.4789, total: 904.7075, cumulativeTotal: 70921.2479 },
    { price: 1889.49, amount: 1.203, total: 2272.9085, cumulativeTotal: 73194.1564 },
    { price: 1889.11, amount: 2.156, total: 4074.4232, cumulativeTotal: 77268.5795 },
    { price: 1888.76, amount: 0.7891, total: 1490.7798, cumulativeTotal: 78759.3593 },
    { price: 1888.43, amount: 1.9503, total: 3683.1898, cumulativeTotal: 82442.5491 },
    { price: 1888.15, amount: 1.0567, total: 1995.3696, cumulativeTotal: 84437.9187 },
    { price: 1887.73, amount: 0.2231, total: 421.1816, cumulativeTotal: 84859.1003 },
    { price: 1887.49, amount: 1.765, total: 3332.1448, cumulativeTotal: 88191.2451 },
    { price: 1887.08, amount: 1.3302, total: 2510.9162, cumulativeTotal: 90702.1613 },
    { price: 1886.81, amount: 0.5876, total: 1109.0091, cumulativeTotal: 91811.1704 },
    { price: 1886.42, amount: 2.201, total: 4152.8108, cumulativeTotal: 95963.9812 },
    { price: 1886.17, amount: 0.88, total: 1660.0296, cumulativeTotal: 97624.0108 },
    { price: 1885.79, amount: 1.4521, total: 2738.4087, cumulativeTotal: 100362.4195 },
    { price: 1885.55, amount: 1.1987, total: 2260.2264, cumulativeTotal: 102622.6459 },
    { price: 1885.23, amount: 0.7012, total: 1322.0072, cumulativeTotal: 103944.6531 },
    { price: 1884.88, amount: 2.35, total: 4430.468, cumulativeTotal: 108375.1211 },
    { price: 1884.51, amount: 1.002, total: 1888.279, cumulativeTotal: 110263.4001 },
    { price: 1884.29, amount: 0.4005, total: 754.6581, cumulativeTotal: 111018.0582 },
    { price: 1883.95, amount: 1.6788, total: 3162.6901, cumulativeTotal: 114180.7483 },
    { price: 1883.62, amount: 1.2501, total: 2354.7134, cumulativeTotal: 116535.4616 },
    { price: 1883.33, amount: 0.808, total: 1521.9306, cumulativeTotal: 118057.3923 },
  ],
  asks: [
    { price: 1899.3, amount: 0.75, total: 1424.475, cumulativeTotal: 1424.475 },
    { price: 1899.4, amount: 1.5, total: 2849.1, cumulativeTotal: 4273.575 },
    { price: 1900.0, amount: 2.1, total: 3990.0, cumulativeTotal: 8263.575 },
    { price: 1900.2, amount: 0.4, total: 760.08, cumulativeTotal: 9023.655 },
    { price: 1900.5, amount: 1.2, total: 2280.6, cumulativeTotal: 11304.255 },
    { price: 1900.8, amount: 0.65, total: 1235.52, cumulativeTotal: 12539.775 },
    { price: 1901.1, amount: 1.7, total: 3231.87, cumulativeTotal: 15771.645 },
    { price: 1901.5, amount: 0.9, total: 1711.35, cumulativeTotal: 17482.995 },
    { price: 1901.9, amount: 2.3, total: 4374.37, cumulativeTotal: 21857.365 },
    { price: 1902.3, amount: 0.55, total: 1046.265, cumulativeTotal: 22903.63 },
    { price: 1902.7, amount: 1.0, total: 1902.7, cumulativeTotal: 24806.33 },
    { price: 1903.0, amount: 2.8, total: 5328.4, cumulativeTotal: 30134.73 },
    { price: 1903.4, amount: 0.3, total: 571.02, cumulativeTotal: 30705.75 },
    { price: 1903.8, amount: 1.4, total: 2665.32, cumulativeTotal: 33371.07 },
    { price: 1904.2, amount: 0.8, total: 1523.36, cumulativeTotal: 34894.43 },
    { price: 1904.6, amount: 1.95, total: 3713.97, cumulativeTotal: 38608.4 },
    { price: 1905.0, amount: 0.25, total: 476.25, cumulativeTotal: 39084.65 },
    { price: 1905.5, amount: 1.6, total: 3048.8, cumulativeTotal: 42133.45 },
    { price: 1905.9, amount: 0.95, total: 1810.605, cumulativeTotal: 43944.055 },
    { price: 1906.4, amount: 2.0, total: 3812.8, cumulativeTotal: 47756.855 },
    { price: 1906.73, amount: 1.4203, total: 2708.1768, cumulativeTotal: 50465.0318 },
    { price: 1907.01, amount: 0.8812, total: 1680.8488, cumulativeTotal: 52145.8806 },
    { price: 1907.35, amount: 2.153, total: 4108.0698, cumulativeTotal: 56253.9503 },
    { price: 1907.77, amount: 1.0321, total: 1969.0032, cumulativeTotal: 58222.9535 },
    { price: 1908.02, amount: 0.5011, total: 956.1296, cumulativeTotal: 59179.0831 },
    { price: 1908.33, amount: 1.7589, total: 3356.7086, cumulativeTotal: 62535.7917 },
    { price: 1908.78, amount: 0.9992, total: 1907.2141, cumulativeTotal: 64443.0058 },
    { price: 1909.11, amount: 2.4001, total: 4582.0549, cumulativeTotal: 69025.0607 },
    { price: 1909.45, amount: 1.234, total: 2356.2433, cumulativeTotal: 71381.304 },
    { price: 1909.88, amount: 0.67, total: 1279.6196, cumulativeTotal: 72660.9236 },
    { price: 1910.21, amount: 1.801, total: 3440.2882, cumulativeTotal: 76101.2118 },
    { price: 1910.55, amount: 1.1023, total: 2106.0764, cumulativeTotal: 78207.2882 },
    { price: 1910.93, amount: 0.3456, total: 660.4159, cumulativeTotal: 78867.7041 },
    { price: 1911.22, amount: 2.05, total: 3918.001, cumulativeTotal: 82785.7051 },
    { price: 1911.67, amount: 1.3005, total: 2485.2293, cumulativeTotal: 85270.9344 },
    { price: 1911.99, amount: 0.7521, total: 1437.9983, cumulativeTotal: 86708.9327 },
    { price: 1912.31, amount: 1.987, total: 3799.831, cumulativeTotal: 90508.7636 },
    { price: 1912.74, amount: 1.0012, total: 1915.0186, cumulativeTotal: 92423.7823 },
    { price: 1913.03, amount: 0.4588, total: 877.6577, cumulativeTotal: 93301.44 },
    { price: 1913.48, amount: 2.2503, total: 4305.7496, cumulativeTotal: 97607.1897 },
    { price: 1913.81, amount: 1.156, total: 2212.0844, cumulativeTotal: 99819.274 },
    { price: 1914.23, amount: 0.8, total: 1531.384, cumulativeTotal: 101350.658 },
    { price: 1914.55, amount: 1.602, total: 3067.1591, cumulativeTotal: 104417.8171 },
    { price: 1914.92, amount: 1.255, total: 2401.9246, cumulativeTotal: 106819.7417 },
    { price: 1915.28, amount: 0.903, total: 1729.5986, cumulativeTotal: 108549.3404 },
    { price: 1915.67, amount: 2.01, total: 3850.4967, cumulativeTotal: 112399.8371 },
    { price: 1916.05, amount: 1.35, total: 2586.6675, cumulativeTotal: 114986.5046 },
    { price: 1916.33, amount: 0.555, total: 1063.5632, cumulativeTotal: 116050.0677 },
    { price: 1916.78, amount: 1.705, total: 3268.0099, cumulativeTotal: 119318.0776 },
    { price: 1917.11, amount: 1.022, total: 1959.2864, cumulativeTotal: 121277.364 },
  ],
};

const OrderRowForImage: React.FC<
  OrderBookRow & {
    type: "bid" | "ask";
    maxCumulativeTotal: number;
  }
> = ({ price, amount, total, cumulativeTotal, maxCumulativeTotal, type }) => {
  const depthBarWidthPercent =
    maxCumulativeTotal > 0 ? (cumulativeTotal / maxCumulativeTotal) * 100 : 0;

  const depthBarClass =
    type === "bid" ? "bg-green-300 left-0" : "bg-red-300 right-0 lg:left-0 lg:right-auto";

  return (
    <div className="relative flex justify-between diatype-xs-medium text-gray-700">
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
      <div className="z-10 text-center">{amount.toFixed(4)}</div>
      <div className="z-10 text-right">{total.toFixed(2)}</div>
    </div>
  );
};

export const OrderBook: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"order book" | "trades">("order book");
  const { bids, asks } = mockOrderBookData;
  const maxCumulativeAsk = asks.length > 0 ? asks[asks.length - 1].cumulativeTotal : 0;
  const maxCumulativeBid = bids.length > 0 ? bids[bids.length - 1].cumulativeTotal : 0;

  const groupedAsks = groupOrdersByPrice(mockOrderBookData.asks).slice(0, 10);
  const groupedBids = groupOrdersByPrice(mockOrderBookData.bids).slice(0, 10);

  return (
    <div className="p-4 shadow-card-shadow bg-rice-25 flex flex-col gap-2 min-w-[20rem]">
      <Tabs
        color="line-red"
        layoutId="tabs-order-history"
        selectedTab={activeTab}
        keys={["order book", "trades"]}
        fullWidth
        onTabChange={(tab) => setActiveTab(tab as "order book" | "trades")}
      />
      {activeTab === "order book" ? (
        <div className="flex gap-2 lg:flex-col">
          <div className="asks-container flex flex-col">
            {groupedAsks.slice().map((ask) => (
              <OrderRowForImage
                key={`ask-${ask.price}`}
                {...ask}
                type="ask"
                maxCumulativeTotal={maxCumulativeAsk}
              />
            ))}
          </div>
          {bids.length > 0 && asks.length > 0 && (
            <div className="diatype-xs-medium text-gray-500 hidden lg:block">
              Spread:{" "}
              {(
                asks.reduce((min, p) => (p.price < min ? p.price : min), asks[0].price) -
                bids[0].price
              ).toFixed(1)}
            </div>
          )}
          <div className="bid-container flex flex-col">
            {groupedBids
              .slice()
              .sort((a, b) => b.price - a.price)
              .map((bid) => (
                <OrderRowForImage
                  key={`bid-${bid.price}`}
                  {...bid}
                  type="bid"
                  maxCumulativeTotal={maxCumulativeAsk}
                />
              ))}
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-2">Life Trades</div>
      )}
    </div>
  );
};
