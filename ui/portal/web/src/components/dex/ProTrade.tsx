import {
  Badge,
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
import {
  useConfig,
  usePublicClient,
  useAccount,
  useOrderBookState,
  useLiveTradesState,
  usePerpsUserState,
  useOrdersByUser,
  usePerpsOrdersByUser,
  perpsOrdersByUserStore,
  perpsUserStateStore,
  usePrices,
  tradePairStore,
  tradeInfoStore,
  useTradeCoins,
  useQueryWithPagination,
} from "@left-curve/store";
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
import { TimeInForceOption, type OrderId, type PairId, type Trade } from "@left-curve/dango/types";

const [ProTradeProvider, useProTrade] = createContext<{
  controllers: ReturnType<typeof useInputs>;
  onChangePairId: (pairSymbols: string, type: "spot" | "perps") => void;
}>({
  name: "ProTradeContext",
});

export { useProTrade };

const TradeSubscriptions: React.FC = () => {
  const mode = tradePairStore((s) => s.mode);
  const pairId = tradePairStore((s) => s.pairId);

  useOrderBookState({ pairId, subscribe: mode === "spot" });
  useLiveTradesState({ pairId, subscribe: mode === "spot" });

  usePerpsUserState({ subscribe: mode === "perps" });
  usePerpsOrdersByUser({ subscribe: mode === "perps" });

  return null;
};

type ProTradeProps = {
  action: "buy" | "sell";
  onChangeAction: (action: "buy" | "sell") => void;
  pairId: PairId;
  onChangePairId: (pairSymbols: string, type: "spot" | "perps") => void;
  orderType: "limit" | "market";
  onChangeOrderType: (orderType: "limit" | "market") => void;
  type?: "spot" | "perps";
};

const ProTradeContainer: React.FC<PropsWithChildren<ProTradeProps>> = ({
  pairId,
  action,
  onChangePairId,
  onChangeAction,
  orderType,
  onChangeOrderType,
  type = "spot",
  children,
}) => {
  const controllers = useInputs();

  useEffect(() => {
    tradePairStore.getState().setPair(pairId, type);
  }, [pairId, type]);

  useEffect(() => {
    tradeInfoStore.getState().setAction(action);
  }, [action]);

  useEffect(() => {
    tradeInfoStore.getState().setOperation(orderType);
  }, [orderType]);

  useEffect(() => {
    return tradeInfoStore.subscribe((state, prev) => {
      if (state.action !== prev.action) onChangeAction(state.action);
      if (state.operation !== prev.operation) onChangeOrderType(state.operation);
    });
  }, [onChangeAction, onChangeOrderType]);

  return (
    <ProTradeProvider value={{ controllers, onChangePairId }}>
      <TradeSubscriptions />
      {children}
    </ProTradeProvider>
  );
};

const ProTradeHeader: React.FC = () => {
  return <TradeHeader />;
};

const ProTradeOverview: React.FC = () => {
  const { controllers } = useProTrade();
  return <OrderBookOverview controllers={controllers} />;
};

const TradingView = lazy(() =>
  import("./TradingView").then(({ TradingView }) => ({ default: TradingView })),
);

const ProTradeChart: React.FC = () => {
  const mode = tradePairStore((s) => s.mode);
  const pairId = tradePairStore((s) => s.pairId);
  const { isLg } = useMediaQuery();

  const { baseCoin, quoteCoin } = useTradeCoins({ pairId, mode });

  const orders = useOrdersByUser({ enabled: mode === "spot", initialData: [] });

  const mobileContainer = usePortalTarget("#chart-container-mobile");

  const ordersByPair = useMemo(
    () =>
      mode === "perps"
        ? []
        : orders.data
          ? orders.data.filter(
              (o) => o.baseDenom === baseCoin.denom && o.quoteDenom === quoteCoin.denom,
            )
          : [],
    [mode, orders.data, baseCoin.denom, quoteCoin.denom],
  );

  if (mode === "perps") {
    return (
      <div className="flex w-full lg:min-h-[32.875rem] h-full items-center justify-center bg-surface-primary-rice">
        <p className="diatype-m-regular text-ink-tertiary-500">Perps chart available soon</p>
      </div>
    );
  }

  const Chart = (
    <Suspense fallback={<Spinner color="pink" size="md" />}>
      <div className="flex w-full lg:min-h-[32.875rem] h-full" id="chart-container">
        <ErrorBoundary fallback={<div className="p-4">Chart Engine</div>}>
          <TradingView coins={{ base: baseCoin, quote: quoteCoin }} orders={ordersByPair} />
        </ErrorBoundary>
      </div>
    </Suspense>
  );

  return <>{isLg ? Chart : mobileContainer ? createPortal(Chart, mobileContainer) : null}</>;
};

const ProTradeMenu: React.FC = () => {
  const { isLg } = useMediaQuery();
  const { controllers } = useProTrade();

  return (
    <>
      <TradeMenu controllers={controllers} />
      {!isLg ? <TradeButtons /> : null}
    </>
  );
};

type BottomTab = "positions" | "open-orders" | "trade-history";

const ProTradeHistory: React.FC = () => {
  const mode = tradePairStore((s) => s.mode);
  const defaultTab: BottomTab = mode === "perps" ? "positions" : "open-orders";
  const [activeTab, setActiveTab] = useState<BottomTab>(defaultTab);

  useEffect(() => {
    setActiveTab(mode === "perps" ? "positions" : "open-orders");
  }, [mode]);

  return (
    <div className="flex-1 p-4 bg-surface-primary-rice flex flex-col gap-2 shadow-account-card pb-20 lg:pb-5 z-10">
      <div className="relative">
        <Tabs
          color="line-red"
          layoutId="tabs-open-orders"
          onTabChange={(tab) => setActiveTab(tab as BottomTab)}
          selectedTab={activeTab}
          classNames={{ button: "exposure-xs-italic", base: "z-10" }}
        >
          {mode === "perps" ? <Tab title="positions">Positions</Tab> : null}
          <Tab title="open-orders">{m["dex.protrade.openOrders"]()}</Tab>
          <Tab title="trade-history">{m["dex.protrade.tradeHistory.title"]()}</Tab>
        </Tabs>
        <span className="w-full absolute h-[2px] bg-outline-secondary-gray bottom-[0px] z-0" />
      </div>
      <div className="w-full h-full relative">
        {activeTab === "positions" && mode === "perps" ? <PerpsPositionsTable /> : null}
        {activeTab === "open-orders" ? <UnifiedOpenOrders /> : null}
        {activeTab === "trade-history" ? <ProTradeOrdersHistory /> : null}
      </div>
    </div>
  );
};

type PerpsPositionRow = {
  pairId: string;
  size: string;
  entryPrice: string;
  currentPrice: number;
  pnl: number;
};

const PerpsPositionsTable: React.FC = () => {
  const { showModal, settings } = useApp();
  const { coins } = useConfig();
  const { formatNumberOptions } = settings;
  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  const userState = perpsUserStateStore((s) => s.userState);

  const symbolToDenom = useMemo(() => {
    const map: Record<string, string> = {};
    for (const [denom, coin] of Object.entries(coins.byDenom)) {
      map[coin.symbol.toLowerCase()] = denom;
    }
    return map;
  }, [coins]);

  const rows = useMemo(() => {
    const result: PerpsPositionRow[] = [];
    if (!userState?.positions) return result;

    for (const [pairId, pos] of Object.entries(userState.positions)) {
      const baseSymbol = pairId.replace("perp/", "").replace(/usd$/i, "");
      const baseDenom = symbolToDenom[baseSymbol] ?? baseSymbol;
      const currentPrice = getPrice(1, baseDenom) || Number(pos.entryPrice);
      const size = Number(pos.size);
      const pnl = size * (currentPrice - Number(pos.entryPrice));
      result.push({
        pairId,
        size: pos.size,
        entryPrice: pos.entryPrice,
        currentPrice,
        pnl,
      });
    }
    return result;
  }, [userState, getPrice, symbolToDenom]);

  const columns: TableColumn<PerpsPositionRow> = [
    {
      header: "Pair",
      cell: ({ row }) => {
        const label = row.original.pairId
          .replace("perp/", "")
          .replace(/usd$/i, "/USD")
          .toUpperCase();
        return (
          <div className="flex items-center gap-1">
            <Cell.Text text={label} className="diatype-xs-medium" />
            <Badge text="Perp" color="green" size="s" />
          </div>
        );
      },
    },
    {
      header: "Side",
      cell: ({ row }) => {
        const isLong = Number(row.original.size) > 0;
        return (
          <Cell.Text
            text={isLong ? "Long" : "Short"}
            className={isLong ? "text-utility-success-600" : "text-utility-error-600"}
          />
        );
      },
    },
    {
      header: "Size",
      cell: ({ row }) => (
        <Cell.Number
          formatOptions={formatNumberOptions}
          value={Math.abs(Number(row.original.size)).toString()}
        />
      ),
    },
    {
      header: "Entry Price",
      cell: ({ row }) => (
        <Cell.Text text={`$${formatNumber(row.original.entryPrice, formatNumberOptions)}`} />
      ),
    },
    {
      header: "Mark Price",
      cell: ({ row }) => (
        <Cell.Text
          text={`$${formatNumber(row.original.currentPrice.toString(), formatNumberOptions)}`}
        />
      ),
    },
    {
      header: "PNL",
      cell: ({ row }) => {
        const isPositive = row.original.pnl >= 0;
        return (
          <Cell.Text
            text={`${isPositive ? "+" : ""}$${formatNumber(row.original.pnl.toFixed(2), formatNumberOptions)}`}
            className={isPositive ? "text-utility-success-600" : "text-utility-error-600"}
          />
        );
      },
    },
    {
      id: "close-position",
      header: () => <Cell.Text text="" />,
      cell: ({ row }) => (
        <Cell.Action
          action={() =>
            showModal(Modals.PerpsClosePosition, {
              pairId: row.original.pairId,
              size: row.original.size,
              pnl: row.original.pnl,
            })
          }
          label="Close"
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
      data={rows}
      columns={columns}
      style="simple"
      classNames={{
        row: "h-fit",
        header: "pt-0",
        base: "pb-[1.5rem] max-h-[9.5rem] overflow-y-scroll",
        cell: twMerge("diatype-xs-regular py-1", {
          "group-hover:bg-transparent": !rows.length,
        }),
      }}
      emptyComponent={<EmptyPlaceholder component="No open positions" className="h-[3.5rem]" />}
    />
  );
};

type UnifiedOrder = {
  id: string;
  market: "spot" | "perps";
  pairDisplay: string;
  side: "buy" | "sell";
  type: "limit";
  price: string;
  size: string;
  filled: string | null;
  reduceOnly: boolean;
  rawSpotOrderId?: OrderId;
  rawPerpsOrderId?: string;
};

const UnifiedOpenOrders: React.FC = () => {
  const { showModal, settings } = useApp();
  const { coins } = useConfig();
  const mode = tradePairStore((s) => s.mode);
  const pairId = tradePairStore((s) => s.pairId);
  const { formatNumberOptions } = settings;
  const { baseCoin } = useTradeCoins({ pairId, mode });

  const [showAllPairs, setShowAllPairs] = useState(false);

  const spotOrders = useOrdersByUser({ enabled: mode === "spot", initialData: [] });

  const perpsOrders = perpsOrdersByUserStore((s) => s.orders);

  const currentPerpsPairId = useMemo(() => {
    if (mode !== "perps") return "";
    const base = baseCoin.symbol?.toLowerCase() ?? "";
    return `perp/${base}usd`;
  }, [mode, baseCoin.symbol]);

  const unifiedRows = useMemo(() => {
    const rows: UnifiedOrder[] = [];

    if (mode === "spot" && spotOrders.data) {
      for (const order of spotOrders.data) {
        const baseDecimals = coins.byDenom[order.baseDenom]?.decimals ?? 6;
        const quoteDecimals = coins.byDenom[order.quoteDenom]?.decimals ?? 6;
        const baseSymbol = coins.byDenom[order.baseDenom]?.symbol ?? order.baseDenom;
        const quoteSymbol = coins.byDenom[order.quoteDenom]?.symbol ?? order.quoteDenom;

        const originalSize = Decimal(order.amount).div(Decimal(10).pow(baseDecimals)).toFixed();
        const filledQty = Decimal(order.amount)
          .minus(Decimal(order.remaining))
          .div(Decimal(10).pow(baseDecimals))
          .toFixed();

        rows.push({
          id: order.id,
          market: "spot",
          pairDisplay: `${baseSymbol}/${quoteSymbol}`,
          side: order.direction === "bid" ? "buy" : "sell",
          type: "limit",
          price: formatNumber(
            Decimal(order.price)
              .times(Decimal(10).pow(baseDecimals - quoteDecimals))
              .toFixed(),
            formatNumberOptions,
          ),
          size: originalSize,
          filled: filledQty,
          reduceOnly: false,
          rawSpotOrderId: order.id,
        });
      }
    }

    if (mode === "perps" && perpsOrders) {
      const allPerpsOrders = [...perpsOrders.bids, ...perpsOrders.asks];
      const filtered = showAllPairs
        ? allPerpsOrders
        : allPerpsOrders.filter((o) => o.pairId === currentPerpsPairId);

      for (const order of filtered) {
        const label = order.pairId.replace("perp/", "").replace(/usd$/i, "/USD").toUpperCase();
        const isLong = Number(order.size) > 0;

        rows.push({
          id: order.orderId,
          market: "perps",
          pairDisplay: label,
          side: isLong ? "buy" : "sell",
          type: "limit",
          price: `$${formatNumber(order.limitPrice, formatNumberOptions)}`,
          size: Math.abs(Number(order.size)).toString(),
          filled: null,
          reduceOnly: order.reduceOnly,
          rawPerpsOrderId: order.orderId,
        });
      }
    }

    return rows;
  }, [
    mode,
    spotOrders.data,
    perpsOrders,
    showAllPairs,
    currentPerpsPairId,
    coins,
    formatNumberOptions,
  ]);

  const columns: TableColumn<UnifiedOrder> = [
    {
      header: "Market",
      cell: ({ row }) => (
        <Badge
          text={row.original.market === "spot" ? "Spot" : "Perp"}
          color={row.original.market === "spot" ? "blue" : "green"}
          size="s"
        />
      ),
    },
    {
      header: "Pair",
      cell: ({ row }) => (
        <Cell.Text text={row.original.pairDisplay} className="diatype-xs-medium" />
      ),
    },
    {
      header: "Side",
      cell: ({ row }) => {
        const isBuy = row.original.side === "buy";
        return (
          <Cell.Text
            text={
              row.original.market === "perps" ? (isBuy ? "Long" : "Short") : isBuy ? "Buy" : "Sell"
            }
            className={isBuy ? "text-utility-success-600" : "text-utility-error-600"}
          />
        );
      },
    },
    {
      header: "Type",
      cell: () => <Cell.Text text="Limit" />,
    },
    {
      header: "Price",
      cell: ({ row }) => <Cell.Text text={row.original.price} />,
    },
    {
      header: "Size",
      cell: ({ row }) => (
        <Cell.Number formatOptions={formatNumberOptions} value={row.original.size} />
      ),
    },
    {
      header: "Filled",
      cell: ({ row }) =>
        row.original.filled !== null ? (
          <Cell.Number formatOptions={formatNumberOptions} value={row.original.filled} />
        ) : (
          <Cell.Text text="-" className="text-ink-tertiary-500" />
        ),
    },
    ...(mode === "perps"
      ? [
          {
            header: "Reduce Only",
            cell: ({ row }: { row: { original: UnifiedOrder } }) =>
              row.original.reduceOnly ? (
                <Badge text="Yes" color="warning" size="s" />
              ) : (
                <Cell.Text text="-" className="text-ink-tertiary-500" />
              ),
          },
        ]
      : []),
    {
      id: "cancel-order",
      header: () => (
        <Cell.Action
          isDisabled={!unifiedRows.length}
          action={() => {
            if (mode === "spot") {
              const ids = unifiedRows.filter((o) => o.rawSpotOrderId).map((o) => o.rawSpotOrderId!);
              showModal(Modals.ProTradeCloseAll, { ordersId: ids });
            } else {
              showModal(Modals.PerpsCloseAll, {});
            }
          }}
          label={m["common.cancelAll"]()}
          classNames={{
            cell: "items-end diatype-xs-regular",
            button: "!exposure-xs-italic m-0 p-0 px-1 h-fit",
          }}
        />
      ),
      cell: ({ row }) => (
        <Cell.Action
          action={() => {
            if (row.original.market === "spot" && row.original.rawSpotOrderId) {
              showModal(Modals.ProTradeCloseOrder, { orderId: row.original.rawSpotOrderId });
            } else if (row.original.rawPerpsOrderId) {
              showModal(Modals.PerpsCloseOrder, { orderId: row.original.rawPerpsOrderId });
            }
          }}
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
    <div className="flex flex-col gap-2">
      {mode === "perps" ? (
        <div className="flex justify-end">
          <label className="flex items-center gap-1.5 cursor-pointer diatype-xs-regular text-ink-tertiary-500">
            <input
              type="checkbox"
              checked={showAllPairs}
              onChange={(e) => setShowAllPairs(e.target.checked)}
              className="accent-primitives-red-light-500 w-3 h-3"
            />
            Show all pairs
          </label>
        </div>
      ) : null}
      <Table
        data={unifiedRows}
        columns={columns}
        style="simple"
        classNames={{
          row: "h-fit",
          header: "pt-0",
          base: "pb-[1.5rem] max-h-[9.5rem] overflow-y-scroll",
          cell: twMerge("diatype-xs-regular py-1", {
            "group-hover:bg-transparent": !unifiedRows.length,
          }),
        }}
        emptyComponent={
          <EmptyPlaceholder
            component={m["dex.protrade.history.noOpenOrders"]()}
            className="h-[3.5rem]"
          />
        }
      />
    </div>
  );
};

const ProTradeOrdersHistory: React.FC = () => {
  const navigate = useNavigate();
  const { settings } = useApp();
  const { coins } = useConfig();
  const { account } = useAccount();
  const publicClient = usePublicClient();
  const mode = tradePairStore((s) => s.mode);
  const pairId = tradePairStore((s) => s.pairId);

  const { baseCoin } = useTradeCoins({ pairId, mode });
  const { formatNumberOptions } = settings;

  const history = useQueryWithPagination({
    enabled: !!account && mode === "spot",
    queryKey: ["tradeHistory", account?.address as string],
    queryFn: async () => {
      if (!account) throw new Error();
      return await publicClient.queryTrades({ address: account.address });
    },
  });

  const { data, pagination, isLoading } = history;

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

  if (mode === "perps") {
    return (
      <EmptyPlaceholder
        component={m["dex.protrade.history.noOpenOrders"]()}
        className="h-[3.5rem]"
      />
    );
  }

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
