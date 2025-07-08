import {
  AddressVisualizer,
  createContext,
  twMerge,
  useInputs,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { useAppConfig, useConfig, usePrices, useProTradeState } from "@left-curve/store";
import { useAccount, useSigningClient } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { useApp } from "~/hooks/useApp";

import { formatUnits, parseUnits } from "@left-curve/dango/utils";
import { m } from "~/paraglide/messages";

import {
  Badge,
  Cell,
  IconChevronDownFill,
  IconEmptyStar,
  Table,
  Tabs,
} from "@left-curve/applets-kit";
import { EmptyPlaceholder } from "../foundation/EmptyPlaceholder";
import { AnimatePresence, motion } from "framer-motion";
import { OrderBookOverview } from "./OrderBookOverview";
import { SearchToken } from "./SearchToken";
import { TradeMenu } from "./TradeMenu";
import { TradingViewChart } from "./TradingViewChart";

import type { TableColumn } from "@left-curve/applets-kit";
import type { OrdersByUserResponse, PairId } from "@left-curve/dango/types";
import type { PropsWithChildren } from "react";

const [ProTradeProvider, useProTrade] = createContext<{
  state: ReturnType<typeof useProTradeState>;
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "ProTradeContext",
});

type ProTradeProps = {
  action: "buy" | "sell";
  onChangeAction: (action: "buy" | "sell") => void;
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
};

const ProTradeContainer: React.FC<PropsWithChildren<ProTradeProps>> = ({
  action,
  onChangeAction,
  pairId,
  onChangePairId,
  children,
}) => {
  const controllers = useInputs();
  const state = useProTradeState({
    controllers,
    pairId,
    onChangePairId,
    action,
    onChangeAction,
  });
  return <ProTradeProvider value={{ state, controllers }}>{children}</ProTradeProvider>;
};

const ProTradeHeader: React.FC = () => {
  const { data: config } = useAppConfig();
  const { isLg } = useMediaQuery();
  const [isExpanded, setIsExpanded] = useState(isLg);
  const { state } = useProTrade();
  const { pairId, onChangePairId } = state;
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  const navigate = useNavigate();

  useEffect(() => {
    setIsExpanded(isLg);
  }, [isLg]);

  return (
    <div className="flex bg-rice-50 lg:gap-8 p-4 flex-col lg:flex-row w-full lg:justify-between">
      <div className="flex gap-8 items-center justify-between lg:items-start w-full lg:w-auto">
        <div className="flex lg:flex-col gap-2">
          <SearchToken pairId={pairId} onChangePairId={onChangePairId} />
          <div className="lg:pl-8">
            <Badge text="Spot" color="blue" size="s" />
          </div>
        </div>
        <div className="flex gap-2 items-center">
          <div
            className="cursor-pointer flex items-center justify-center lg:hidden"
            onClick={() => setIsExpanded(!isExpanded)}
          >
            <IconChevronDownFill
              className={twMerge("text-gray-500 w-4 h-4 transition-all", {
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
            layout="position"
            layoutId="protrade-header"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: isLg ? 0 : 0.3, ease: "easeInOut" }}
            className="gap-2 lg:gap-5 grid grid-cols-1 lg:flex lg:flex-wrap lg:items-center overflow-hidden"
          >
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start pt-8 lg:pt-0">
              <p className="diatype-xs-medium text-gray-500">{m["dex.protrade.spot.price"]()}</p>
              <p className="diatype-sm-bold text-gray-700">
                {getPrice(1, pairId.baseDenom, { format: true })}
              </p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-gray-500">
                {m["dex.protrade.spot.24hChange"]()}
              </p>
              <p className="diatype-sm-bold w-full text-center">-</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-gray-500">{m["dex.protrade.spot.volume"]()}</p>
              <p className="diatype-sm-bold w-full text-center">-</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-gray-500">{m["dex.contract"]()}</p>
              <AddressVisualizer
                address={config?.addresses.dex || "0x"}
                withIcon
                onClick={(url) => navigate({ to: url })}
                classNames={{ text: "diatype-sm-bold" }}
              />
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
  const { state, controllers } = useProTrade();
  return <TradeMenu state={state} controllers={controllers} />;
};

const ProTradeOrders: React.FC = () => {
  const { showModal: _ } = useApp();
  const { coins } = useConfig();
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const [activeTab, setActiveTab] = useState<"open order" | "trade history">("open order");

  const { state } = useProTrade();
  const { orders } = state;

  const columns: TableColumn<OrdersByUserResponse & { id: number }> = [
    /*  {
      header: "Time",
      cell: ({ row }) => <Cell.Time date={row.original.time} />,
    }, */
    {
      header: m["dex.protrade.spot.ordersTable.type"](),
      cell: ({ row }) => <Cell.Text text="Limit" />,
    },
    {
      header: m["dex.protrade.spot.ordersTable.coin"](),
      cell: ({ row }) => {
        return <Cell.Asset noImage denom={row.original.baseDenom} />;
      },
    },
    {
      header: m["dex.protrade.spot.ordersTable.direction"](),
      cell: ({ row }) => (
        <Cell.OrderDirection
          text={m["dex.protrade.spot.direction"]({ direction: row.original.direction })}
          direction={row.original.direction}
        />
      ),
    },
    {
      header: m["dex.protrade.spot.ordersTable.size"](),
      cell: ({ row }) => (
        <Cell.Text
          text={formatUnits(row.original.remaining, coins[row.original.baseDenom].decimals)}
        />
      ),
    },
    {
      header: m["dex.protrade.spot.ordersTable.originalSize"](),
      cell: ({ row }) => (
        <Cell.Text
          text={formatUnits(row.original.amount, coins[row.original.baseDenom].decimals)}
        />
      ),
    },
    {
      header: m["dex.protrade.spot.price"](),
      cell: ({ row }) => (
        <Cell.Text
          text={parseUnits(
            row.original.price,
            coins[row.original.baseDenom].decimals - coins[row.original.quoteDenom].decimals,
          ).toString()}
        />
      ),
    },
    {
      id: "cancel-order",
      header: () => (
        <Cell.Action
          isDisabled={!orders.data.length}
          action={() =>
            signingClient?.batchUpdateOrders({ cancels: "all", sender: account!.address })
          }
          label={m["common.cancelAll"]()}
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
          label={m["common.cancel"]()}
          classNames={{ cell: "items-end", button: "!exposure-xs-italic m-0 p-0 px-1 h-fit" }}
        />
      ),
    },
  ];

  return (
    <div className="flex-1 p-4 bg-rice-25 flex flex-col gap-2 shadow-card-shadow pb-20 lg:pb-5 z-10">
      <div className="relative">
        <Tabs
          color="line-red"
          layoutId="tabs-open-order"
          selectedTab={activeTab}
          keys={["open order", "trade history"]}
          onTabChange={(tab) => setActiveTab(tab as "open order" | "trade history")}
          classNames={{ button: "exposure-xs-italic" }}
        />

        <span className="w-full absolute h-[1px] bg-gray-100 bottom-[1px]" />
      </div>
      <div className="w-full h-full relative">
        {activeTab === "open order" ? (
          <Table
            data={orders.data}
            columns={columns}
            style="simple"
            classNames={{
              row: "h-fit",
              header: "pt-0",
              base: "pb-0",
              cell: twMerge("diatype-xs-regular", {
                "group-hover:bg-transparent": !orders.data.length,
              }),
            }}
            emptyComponent={
              activeTab === "open order" ? (
                <EmptyPlaceholder
                  component={m["dex.protrade.spot.noOpenOrders"]()}
                  className="h-[3.5rem]"
                />
              ) : null
            }
          />
        ) : null}
      </div>
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
