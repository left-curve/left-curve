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
import { lazy, Suspense, useMemo, useState } from "react";
import { useConfig, useProTradeState } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { createPortal } from "react-dom";
import { calculateTradeSize, Decimal, formatNumber } from "@left-curve/dango/utils";

import { Cell, Table, Tabs } from "@left-curve/applets-kit";
import { EmptyPlaceholder } from "../foundation/EmptyPlaceholder";
import { OrderBookOverview } from "./OrderBookOverview";
import { TradeButtons } from "./TradeButtons";
import { TradeMenu } from "./TradeMenu";
import { TradeHeader } from "./TradeHeader";
import { ErrorBoundary } from "react-error-boundary";

import type { PropsWithChildren } from "react";
import type { TableColumn } from "@left-curve/applets-kit";
import {
  TimeInForceOption,
  type OrderId,
  type OrdersByUserResponse,
  type PairId,
  type Trade,
} from "@left-curve/dango/types";

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
  const { toast } = useApp();

  const { isLg } = useMediaQuery();

  const state = useProTradeState({
    m,
    controllers,
    pairId,
    bucketRecords: isLg ? 11 : 16,
    onChangePairId,
    action,
    onChangeAction,
    orderType,
    onChangeOrderType,
    submission: {
      onError: (err) => {
        const message = (() => {
          if (err instanceof Error) return err.message;
          if (typeof err === "string") {
            const contractError = err.match(/msg: (.*?),"backtrace":/);
            if (contractError?.[1]) return contractError[1];
          }
          return m["errors.failureRequest"]();
        })();
        toast.error({
          title: m["dex.protrade.orderFailed"](),
          description: message,
        });
      },
    },
  });

  return <ProTradeProvider value={{ state, controllers }}>{children}</ProTradeProvider>;
};

const ProTradeHeader: React.FC = () => {
  const { state } = useProTrade();

  return <TradeHeader state={state} />;
};

const ProTradeOverview: React.FC = () => {
  const { state } = useProTrade();
  return <OrderBookOverview state={state} />;
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
      <div className="flex w-full lg:min-h-[45vh] h-full" id="chart-container">
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
    <div className="flex-1 p-4 bg-surface-primary-rice flex flex-col gap-2 shadow-account-card pb-20 lg:pb-5 z-10">
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
        <span className="w-full absolute h-[2px] bg-outline-secondary-gray bottom-[0px] z-0" />
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
            className="diatype-xs-regular text-ink-secondary-700 hover:text-ink-primary-900 cursor-pointer"
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
          formatOptions={formatNumberOptions}
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
          formatOptions={formatNumberOptions}
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
            formatNumberOptions,
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
        base: "pb-0 max-h-[31vh] overflow-y-scroll",
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
        <Cell.Text
          text={m["dex.protrade.orderType"]({
            orderType:
              row.original.timeInForce === TimeInForceOption.GoodTilCanceled ? "limit" : "market",
          })}
        />
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
            formatOptions={formatNumberOptions}
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
            formatNumberOptions,
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
        base: "pb-0 max-h-[31vh] overflow-y-scroll",
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
