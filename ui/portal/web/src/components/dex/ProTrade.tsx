import { createContext, twMerge, useInputs, useMediaQuery } from "@left-curve/applets-kit";
import { useProTrade } from "@left-curve/store";
import { useAccount, useSigningClient } from "@left-curve/store";
import { useEffect, useMemo, useState } from "react";

import { m } from "~/paraglide/messages";

import { Badge, Cell, IconChevronDown, IconEmptyStar, Table, Tabs } from "@left-curve/applets-kit";
import { AnimatePresence, motion } from "framer-motion";
import { OrderBookOverview } from "./OrderBookOverview";
import { SearchToken } from "./SearchToken";
import { TradeMenu } from "./TradeMenu";
import { TradingViewChart } from "./TradingViewChart";

import type { TableColumn } from "@left-curve/applets-kit";
import type { OrdersByUserResponse } from "@left-curve/dango/types";
import type { PropsWithChildren } from "react";

const [ProTradeProvider, useProTradeState] = createContext<{
  state: ReturnType<typeof useProTrade>;
}>({
  name: "ProTradeContext",
});

const ProTradeContainer: React.FC<PropsWithChildren> = ({ children }) => {
  const __state__ = useProTrade({});
  return (
    <ProTradeProvider
      value={{
        state: __state__,
      }}
    >
      {children}
    </ProTradeProvider>
  );
};

const ProTradeHeader: React.FC = () => {
  const { isLg } = useMediaQuery();
  const [isExpanded, setIsExpanded] = useState(isLg);

  useEffect(() => {
    setIsExpanded(isLg);
  }, [isLg]);

  return (
    <div className="flex bg-rice-50 lg:gap-8 p-4 flex-col lg:flex-row w-full lg:justify-between">
      <div className="flex gap-8 items-center justify-between lg:items-start w-full lg:w-auto">
        <div className="flex lg:flex-col gap-2">
          <SearchToken />
          <div className="lg:pl-8">
            <Badge text="Spot" color="blue" size="s" />
          </div>
        </div>
        <div className="flex gap-2 items-center">
          <div
            className="cursor-pointer flex items-center justify-center lg:hidden"
            onClick={() => setIsExpanded(!isExpanded)}
          >
            <IconChevronDown
              className={twMerge("text-gray-500 w-5 h-5 transition-all", {
                "rotate-180": isExpanded,
              })}
            />
          </div>
          <IconEmptyStar className="w-5 h-5 text-gray-500" />
        </div>
      </div>
      <AnimatePresence initial={false}>
        {isExpanded ? (
          <motion.div
            layout
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: "easeInOut" }}
            className="gap-2 lg:gap-5 grid grid-cols-1 lg:flex lg:flex-wrap lg:items-center overflow-hidden"
          >
            <div className="items-center flex gap-1 flex-row lg:flex-col lg:items-start pt-8 lg:pt-0">
              <p className="diatype-xs-medium text-gray-500">Mark</p>
              <p className="diatype-sm-bold text-gray-700">83,565</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col lg:items-start">
              <p className="diatype-xs-medium text-gray-500">Last price</p>
              <p className="diatype-sm-bold text-gray-700">$2,578</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col lg:items-start">
              <p className="diatype-xs-medium text-gray-500">Oracle</p>
              <p className="diatype-sm-bold text-gray-700">83,565</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col lg:items-start">
              <p className="diatype-xs-medium text-gray-500">24h Change</p>
              <p className="diatype-sm-bold text-red-bean-400">-542 / 0.70</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col lg:items-start">
              <p className="diatype-xs-medium text-gray-500">24h Volume</p>
              <p className="diatype-sm-bold text-gray-700">$2,457,770,700.50</p>
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
};

const ProTradeChart: React.FC = () => {
  const { isLg } = useMediaQuery();

  if (!isLg) return null;

  return (
    <div className="shadow-card-shadow bg-rice-25 h-full">
      <TradingViewChart />
    </div>
  );
};

const ProTradeMenu: React.FC = () => {
  return <TradeMenu />;
};

const ProTradeOrders: React.FC = () => {
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const [activeTab, setActiveTab] = useState<"open order" | "trade history">("open order");

  const { state } = useProTradeState();
  const { orders } = state;

  console.log("Orders:", orders.data.length);

  const columns: TableColumn<OrdersByUserResponse & { id: number }> = [
    /*  {
      header: "Time",
      cell: ({ row }) => <Cell.Time date={row.original.time} />,
    }, */
    {
      header: "Type",
      cell: ({ row }) => <Cell.Text text="Limit" />,
    },
    {
      header: "Coin",
      cell: ({ row }) => {
        return <Cell.Asset noImage denom={row.original.baseDenom} />;
      },
    },
    {
      header: "Direction",
      cell: ({ row }) => (
        <Cell.OrderDirection
          text={m["dex.protrade.spot.direction"]({ direction: row.original.direction })}
          direction={row.original.direction}
        />
      ),
    },
    {
      header: "Size",
      cell: ({ row }) => <Cell.Text text={row.original.remaining} />,
    },
    {
      header: "Original Size",
      cell: ({ row }) => <Cell.Text text={row.original.amount} />,
    },
    {
      header: "Price",
      cell: ({ row }) => <Cell.Text text={row.original.price} />,
    },
    {
      id: "cancel-order",
      header: () => (
        <Cell.Action
          action={() =>
            signingClient?.batchUpdateOrders({ cancels: "all", sender: account!.address })
          }
          label="Cancel All"
          classNames={{
            cell: "items-end diatype-xs-regular",
            button: "!exposure-xs-italic m-0 p-0 px-1 h-fit ",
          }}
        />
      ),
      cell: ({ row }) => (
        <Cell.Action
          action={() =>
            signingClient?.batchUpdateOrders({
              sender: account!.address,
              cancels: { some: [row.original.id] },
            })
          }
          label="Cancel"
          classNames={{ cell: "items-end", button: "!exposure-xs-italic m-0 p-0 px-1 h-fit" }}
        />
      ),
    },
  ];

  return (
    <div className="flex-1 p-4 !pr-2 bg-rice-25 flex flex-col gap-2 shadow-card-shadow pb-20 lg:pb-0 z-10">
      <div className="relative">
        <Tabs
          color="line-red"
          layoutId="tabs-open-order"
          selectedTab={activeTab}
          keys={["open order", "trade history"]}
          onTabChange={(tab) => setActiveTab(tab as "open order" | "trade history")}
          classNames={{ button: "exposure-xs-italic" }}
        />

        <span className="w-full absolute h-[1px] bg-gray-100 bottom-[0.25rem]" />
      </div>
      {activeTab === "open order" ? (
        <Table
          data={
            orders.data.length
              ? orders.data
              : [
                  {
                    id: 0,
                    baseDenom: "",
                    quoteDenom: "",
                    direction: 0,
                    price: "",
                    amount: "",
                    remaining: "",
                  },
                ]
          }
          columns={columns}
          style="simple"
          classNames={{
            row: "h-fit",
            header: "pt-0",
            cell: twMerge("diatype-xs-regular", {
              "group-hover:bg-transparent": !orders.data.length,
            }),
          }}
        />
      ) : null}
      {}
    </div>
  );
};

export const ProTrade = Object.assign(ProTradeContainer, {
  Header: ProTradeHeader,
  Chart: ProTradeChart,
  Orders: ProTradeOrders,
  OrderBook: OrderBookOverview,
  TradeMenu: ProTradeMenu,
});
