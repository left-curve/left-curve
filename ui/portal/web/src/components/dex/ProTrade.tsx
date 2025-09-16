import {
  createContext,
  CursorPagination,
  Modals,
  Spinner,
  Tab,
  twMerge,
  useApp,
  useInputs,
  useMediaQuery,
  usePortalTarget,
} from "@left-curve/applets-kit";
import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import { useConfig, usePrices, useProTradeState } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { createPortal } from "react-dom";
import { calculateTradeSize, Decimal, formatNumber } from "@left-curve/dango/utils";

import { Badge, Cell, IconChevronDownFill, Table, Tabs } from "@left-curve/applets-kit";
import { AnimatePresence, motion } from "framer-motion";
import { EmptyPlaceholder } from "../foundation/EmptyPlaceholder";
import { OrderBookOverview } from "./OrderBookOverview";
import { SearchToken } from "./SearchToken";
import { TradeButtons } from "./TradeButtons";
import { TradeMenu } from "./TradeMenu";
import { ErrorBoundary } from "react-error-boundary";

import type { PropsWithChildren } from "react";
import type { TableColumn } from "@left-curve/applets-kit";
import type { OrderId, OrdersByUserResponse, PairId, Trade } from "@left-curve/dango/types";

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
  orderType: "limit" | "market";
  onChangeOrderType: (orderType: "limit" | "market") => void;
};

const ProTradeContainer: React.FC<PropsWithChildren<ProTradeProps>> = ({
  action,
  onChangeAction,
  pairId,
  onChangePairId,
  orderType,
  onChangeOrderType,
  children,
}) => {
  const controllers = useInputs();

  const state = useProTradeState({
    controllers,
    pairId,
    onChangePairId,
    action,
    onChangeAction,
    orderType,
    onChangeOrderType,
  });

  return <ProTradeProvider value={{ state, controllers }}>{children}</ProTradeProvider>;
};

const ProTradeHeader: React.FC = () => {
  const { isLg } = useMediaQuery();
  const [isExpanded, setIsExpanded] = useState(isLg);
  const { state } = useProTrade();
  const { pairId, onChangePairId } = state;
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { getPrice } = usePrices({
    defaultFormatOptions: { ...formatNumberOptions, maxSignificantDigits: 8 },
  });

  useEffect(() => {
    setIsExpanded(isLg);
  }, [isLg]);

  return (
    <div className="flex bg-surface-tertiary-rice lg:gap-8 p-4 flex-col lg:flex-row w-full lg:justify-between shadow-account-card z-20 lg:z-10">
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
              className={twMerge("text-tertiary-500 w-4 h-4 transition-all", {
                "rotate-180": isExpanded,
              })}
            />
          </div>
          {/*   <IconEmptyStar className="w-5 h-5 text-tertiary-500" /> */}
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
              <p className="diatype-xs-medium text-tertiary-500">
                {m["dex.protrade.history.price"]()}
              </p>
              <p className="diatype-sm-bold text-secondary-700">
                {getPrice(1, pairId.baseDenom, { format: true })}
              </p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-tertiary-500">
                {m["dex.protrade.spot.24hChange"]()}
              </p>
              <p className="diatype-sm-bold w-full text-center">-</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-tertiary-500">
                {m["dex.protrade.spot.volume"]()}
              </p>
              <p className="diatype-sm-bold w-full text-center">-</p>
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
};

const ProTradeOverview: React.FC = () => {
  const { state } = useProTrade();
  const { baseCoin, quoteCoin } = state;
  return <OrderBookOverview base={baseCoin} quote={quoteCoin} />;
};

const ChartIQ = lazy(() =>
  import("../foundation/ChartIQ").then(({ ChartIQ }) => ({ default: ChartIQ })),
);

const TradingView = lazy(() =>
  import("./TradingView").then(({ TradingView }) => ({ default: TradingView })),
);

const ProTradeChart: React.FC = () => {
  const { state } = useProTrade();
  const { isLg } = useMediaQuery();
  const { settings } = useApp();
  const { chart } = settings;
  const { baseCoin, quoteCoin, orders } = state;

  const ChartComponent = chart === "tradingview" ? TradingView : ChartIQ;

  const mobileContainer = usePortalTarget("#chart-container-mobile");

  const ordersByPair = useMemo(
    () =>
      orders.data.filter((o) => o.baseDenom === baseCoin.denom && o.quoteDenom === quoteCoin.denom),
    [orders.data, baseCoin.denom, quoteCoin.denom],
  );

  const Chart = (
    <Suspense fallback={<Spinner color="pink" size="md" />}>
      <div className="flex w-full h-full lg:min-h-[52vh]" id="chart-container">
        <ErrorBoundary fallback={<div className="p-4">Chart Engine</div>}>
          <ChartComponent coins={{ base: baseCoin, quote: quoteCoin }} orders={ordersByPair} />
        </ErrorBoundary>
      </div>
    </Suspense>
  );

  return <>{isLg ? Chart : mobileContainer ? createPortal(Chart, mobileContainer) : null}</>;
};

const ProTradeMenu: React.FC = () => {
  const { isLg } = useMediaQuery();
  const { state, controllers } = useProTrade();

  return (
    <>
      <TradeMenu state={state} controllers={controllers} />
      {!isLg ? <TradeButtons state={state} /> : null}
    </>
  );
};

const ProTradeHistory: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"orders" | "trade-history">("orders");

  return (
    <div className="flex-1 p-4 bg-surface-secondary-rice flex flex-col gap-2 shadow-account-card pb-20 lg:pb-5 z-10">
      <div className="relative">
        <Tabs
          color="line-red"
          layoutId="tabs-open-orders"
          onTabChange={(tab) => setActiveTab(tab as "orders" | "trade-history")}
          selectedTab={activeTab}
          classNames={{ button: "exposure-xs-italic", base: "z-10" }}
        >
          <Tab title="orders">{m["dex.protrade.openOrders"]()}</Tab>
          <Tab title="trade-history">{m["dex.protrade.tradeHistory.title"]()}</Tab>
        </Tabs>
        <span className="w-full absolute h-[2px] bg-secondary-gray bottom-[0px] z-0" />
      </div>
      <div className="w-full h-full relative">
        {activeTab === "orders" ? <ProTradeOpenOrders /> : null}
        {activeTab === "trade-history" ? <ProTradeOrdersHistory /> : null}
      </div>
    </div>
  );
};

const ProTradeOpenOrders: React.FC = () => {
  const { showModal, settings } = useApp();
  const { coins } = useConfig();

  const { state } = useProTrade();
  const { orders, baseCoin } = state;
  const { formatNumberOptions } = settings;

  const columns: TableColumn<OrdersByUserResponse & { id: OrderId }> = [
    /*  {
      header: "Time",
      cell: ({ row }) => <Cell.Time date={row.original.time} />,
    }, */
    {
      header: m["dex.protrade.history.id"](),
      cell: ({ row }) => {
        const orderId = Decimal(row.original.id);
        const value = orderId.gte("9223372036854775807")
          ? Decimal("18446744073709551615").minus(orderId).toString()
          : orderId.toString();
        return (
          <Cell.Text
            text={value}
            className="diatype-xs-regular text-secondary-700 hover:text-primary-900 cursor-pointer"
          />
        );
      },
    },

    {
      header: m["dex.protrade.history.type"](),
      cell: (_) => <Cell.Text text="Limit" />,
    },
    {
      header: m["dex.protrade.history.pair"](),
      cell: ({ row }) => {
        return (
          <div className="flex items-center gap-1">
            <Cell.PairName
              className="diatype-xs-medium"
              pairId={{
                baseDenom: row.original.baseDenom,
                quoteDenom: row.original.quoteDenom,
              }}
            />
          </div>
        );
      },
    },
    {
      header: m["dex.protrade.history.direction"](),
      cell: ({ row }) => (
        <Cell.OrderDirection
          text={m["dex.protrade.spot.direction"]({
            direction: row.original.direction,
          })}
          direction={row.original.direction}
        />
      ),
    },
    {
      id: "remaining",
      header: () =>
        m["dex.protrade.history.remaining"]({
          symbol: baseCoin.symbol,
        }),
      cell: ({ row }) => (
        <Cell.Number
          formatOptions={{ ...formatNumberOptions, maxSignificantDigits: 10 }}
          value={Decimal(row.original.remaining)
            .div(Decimal(10).pow(coins.byDenom[row.original.baseDenom].decimals))
            .toFixed()}
        />
      ),
    },
    {
      id: "size",
      header: () =>
        m["dex.protrade.history.size"]({
          symbol: baseCoin.symbol,
        }),
      cell: ({ row }) => (
        <Cell.Number
          formatOptions={{ ...formatNumberOptions, maxSignificantDigits: 10 }}
          value={Decimal(row.original.amount)
            .div(Decimal(10).pow(coins.byDenom[row.original.baseDenom].decimals))
            .toFixed()}
        />
      ),
    },
    {
      header: m["dex.protrade.history.limitPrice"](),
      cell: ({ row }) => (
        <Cell.Text
          text={formatNumber(
            Decimal(row.original.price)
              .times(
                Decimal(10).pow(
                  coins.byDenom[row.original.baseDenom].decimals -
                    coins.byDenom[row.original.quoteDenom].decimals,
                ),
              )
              .toFixed(),
            { ...formatNumberOptions, maxSignificantDigits: 10 },
          )}
        />
      ),
    },
    {
      id: "cancel-order",
      header: () => (
        <Cell.Action
          isDisabled={!orders.data.length}
          action={() =>
            showModal(Modals.ProTradeCloseAll, { ordersId: orders.data.map((o) => o.id) })
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
          action={() => showModal(Modals.ProTradeCloseOrder, { orderId: row.original.id })}
          label={m["common.cancel"]()}
          classNames={{
            cell: "items-end",
            button: "!exposure-xs-italic m-0 p-0 px-1 h-fit",
          }}
        />
      ),
    },
  ];

  return (
    <Table
      data={orders.data}
      columns={columns}
      style="simple"
      classNames={{
        row: "h-fit",
        header: "pt-0",
        base: "pb-0 max-h-52 overflow-y-scroll",
        cell: twMerge("diatype-xs-regular py-1", {
          "group-hover:bg-transparent": !orders.data.length,
        }),
      }}
      emptyComponent={
        <EmptyPlaceholder
          component={m["dex.protrade.history.noOpenOrders"]()}
          className="h-[3.5rem]"
        />
      }
    />
  );
};

const ProTradeOrdersHistory: React.FC = () => {
  const navigate = useNavigate();
  const { settings } = useApp();
  const { coins } = useConfig();

  const { state } = useProTrade();
  const { history, baseCoin } = state;
  const { data, pagination, isLoading } = history;
  const { formatNumberOptions } = settings;

  const columns: TableColumn<Trade> = [
    {
      header: m["dex.protrade.tradeHistory.pair"](),
      cell: ({ row }) => {
        return (
          <div className="flex items-center gap-1">
            <Cell.PairName
              className="diatype-xs-medium"
              pairId={{
                baseDenom: row.original.baseDenom,
                quoteDenom: row.original.quoteDenom,
              }}
            />
          </div>
        );
      },
    },
    {
      header: m["dex.protrade.tradeHistory.direction"](),
      cell: ({ row }) => (
        <Cell.OrderDirection
          text={m["dex.protrade.spot.direction"]({
            direction: row.original.direction,
          })}
          direction={row.original.direction}
        />
      ),
    },
    {
      header: m["dex.protrade.history.type"](),
      cell: ({ row }) => (
        <Cell.Text text={m["dex.protrade.orderType"]({ orderType: row.original.orderType })} />
      ),
    },
    {
      id: "size",
      header: () =>
        m["dex.protrade.history.size"]({
          symbol: baseCoin.symbol,
        }),
      cell: ({ row }) => {
        return (
          <Cell.Number
            formatOptions={{ ...formatNumberOptions, maxSignificantDigits: 10 }}
            value={calculateTradeSize(
              row.original,
              coins.byDenom[row.original.baseDenom].decimals,
            ).toFixed()}
          />
        );
      },
    },
    {
      header: m["dex.protrade.history.price"](),
      cell: ({ row }) => (
        <Cell.Text
          text={formatNumber(
            Decimal(row.original.clearingPrice)
              .times(
                Decimal(10).pow(
                  coins.byDenom[row.original.baseDenom].decimals -
                    coins.byDenom[row.original.quoteDenom].decimals,
                ),
              )
              .toFixed(),
            { ...formatNumberOptions, maxSignificantDigits: 10 },
          )}
        />
      ),
    },
    {
      header: "Time",
      cell: ({ row }) => <Cell.Time date={row.original.createdAt} dateFormat="MM/dd/yy h:mm a" />,
    },
  ];

  return (
    <Table
      data={data?.nodes || []}
      columns={columns}
      style="simple"
      onRowClick={(row) =>
        navigate({ to: "/block/$block", params: { block: row.original.blockHeight.toString() } })
      }
      classNames={{
        row: "h-fit",
        header: "pt-0",
        base: "pb-0 max-h-52 overflow-y-scroll",
        cell: twMerge("diatype-xs-regular py-1", {
          "group-hover:bg-transparent": !data?.nodes.length,
        }),
      }}
      bottomContent={
        pagination ? (
          <CursorPagination
            {...pagination}
            isLoading={isLoading}
            className="flex w-full justify-end gap-2"
            nextLabel={m["pagination.next"]()}
            previousLabel={m["pagination.previous"]()}
          />
        ) : null
      }
      emptyComponent={
        <EmptyPlaceholder
          component={m["dex.protrade.history.noOpenOrders"]()}
          className="h-[3.5rem]"
        />
      }
    />
  );
};

export const ProTrade = Object.assign(ProTradeContainer, {
  Header: ProTradeHeader,
  Chart: ProTradeChart,
  History: ProTradeHistory,
  OrderBook: ProTradeOverview,
  TradeMenu: ProTradeMenu,
});
