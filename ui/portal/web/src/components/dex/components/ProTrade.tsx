import {
  Badge,
  createContext,
  Modals,
  Spinner,
  Tab,
  twMerge,
  useApp,
  useInputs,
  useMediaQuery,
  usePortalTarget,
} from "@left-curve/applets-kit";
import { lazy, Suspense, useCallback, useEffect, useMemo, useState } from "react";
import {
  useConfig,
  useLivePerpsTradesState,
  usePerpsUserState,
  usePerpsUserStateExtended,
  usePerpsOrdersByUser,
  perpsOrdersByUserStore,
  perpsUserStateStore,
  perpsUserStateExtendedStore,
  TradePairStore,
  tradeInfoStore,
  useTradeCoins,
  usePerpsPairState,
  useAllPerpsPairStats,
  allPerpsPairStatsStore,
  useCurrentPrice,
  useOraclePrices,
} from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { createPortal } from "react-dom";
import { Decimal, formatNumber } from "@left-curve/utils";

import {
  Button,
  Cell,
  FormattedNumber,
  IconShareNodes,
  Table,
  Tabs,
} from "@left-curve/applets-kit";
import { CountBadge } from "../../foundation/CountBadge";
import { EmptyPlaceholder } from "../../foundation/EmptyPlaceholder";
import { OrderBookOverview } from "./OrderBookOverview";
import { TradeButtons } from "./TradeButtons";
import { TradeMenu } from "./TradeMenu";
import { TradeHeader } from "./TradeHeader";
import { ErrorBoundary } from "react-error-boundary";
import { PerpsTradeHistory } from "./TradeHistory";

import type { PropsWithChildren } from "react";
import type { TableColumn } from "@left-curve/applets-kit";
import type { ConditionalOrder, PairId } from "@left-curve/types";

const [ProTradeProvider, useProTrade] = createContext<{
  controllers: ReturnType<typeof useInputs>;
  onChangePairId: (pairSymbols: string) => void;
}>({
  name: "ProTradeContext",
});

export { useProTrade };

const TradeDocumentTitle: React.FC = () => {
  const pairId = TradePairStore((s) => s.pairId);
  const { baseCoin } = useTradeCoins();
  const { currentPrice } = useCurrentPrice();

  useEffect(() => {
    const previousTitle = document.title;
    const symbol = `${baseCoin.symbol}-USD`;

    if (currentPrice) {
      const priceNum = Number(currentPrice);
      const formatted = Number.isFinite(priceNum)
        ? priceNum.toLocaleString(undefined, {
            minimumFractionDigits: 2,
            maximumFractionDigits: priceNum < 1 ? 6 : 2,
          })
        : currentPrice;
      document.title = `${formatted} | ${symbol} | Dango`;
    } else {
      document.title = `${symbol} · Dango`;
    }

    return () => {
      document.title = previousTitle;
    };
  }, [pairId, baseCoin.symbol, currentPrice]);

  return null;
};

const TradeSubscriptions: React.FC = () => {
  useLivePerpsTradesState({ subscribe: true });

  usePerpsUserState({ subscribe: true });
  usePerpsUserStateExtended({ subscribe: true });
  usePerpsPairState({ subscribe: true });
  usePerpsOrdersByUser({ subscribe: true });

  useOraclePrices({ subscribe: true });

  useAllPerpsPairStats();

  return null;
};

type ProTradeProps = {
  action: "buy" | "sell";
  onChangeAction: (action: "buy" | "sell") => void;
  pairId: PairId;
  onChangePairId: (pairSymbols: string) => void;
  orderType: "limit" | "market";
  onChangeOrderType: (orderType: "limit" | "market") => void;
};

const ProTradeContainer: React.FC<PropsWithChildren<ProTradeProps>> = ({
  pairId,
  action,
  onChangePairId,
  onChangeAction,
  orderType,
  onChangeOrderType,
  children,
}) => {
  const controllers = useInputs();

  useEffect(() => {
    TradePairStore.getState().setPair(pairId);
  }, [pairId]);

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

  if (TradePairStore.getState().pairId.baseDenom === "") return null;

  return (
    <ProTradeProvider value={{ controllers, onChangePairId }}>
      <TradeSubscriptions />
      <TradeDocumentTitle />
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
  const { isLg } = useMediaQuery();

  const { baseCoin, quoteCoin } = useTradeCoins();

  const mobileContainer = usePortalTarget("#chart-container-mobile");

  const Chart = (
    <Suspense fallback={<Spinner color="pink" size="md" fullContainer />}>
      <div className="flex w-full lg:min-h-[32.875rem] h-full" id="chart-container">
        <ErrorBoundary fallback={<div className="p-4">Chart Engine</div>}>
          <TradingView coins={{ base: baseCoin, quote: quoteCoin }} />
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
  const [activeTab, setActiveTab] = useState<BottomTab>("positions");

  const userState = perpsUserStateStore((s) => s.userState);
  const perpsOrders = perpsOrdersByUserStore((s) => s.orders);

  const positionsCount = Object.keys(userState?.positions ?? {}).length;
  const openOrdersCount = Object.keys(perpsOrders ?? {}).length;

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
          <Tab title="positions">
            <span className="flex items-center gap-1">
              {m["dex.protrade.positions.title"]()}
              <CountBadge count={positionsCount} />
            </span>
          </Tab>
          <Tab title="open-orders">
            <span className="flex items-center gap-1">
              {m["dex.protrade.openOrders"]()}
              <CountBadge count={openOrdersCount} />
            </span>
          </Tab>
          <Tab title="trade-history">{m["dex.protrade.tradeHistory.title"]()}</Tab>
        </Tabs>
        <span className="w-full absolute h-[2px] bg-outline-secondary-gray bottom-[0px] z-0" />
      </div>
      <div className="w-full h-full relative">
        {activeTab === "positions" ? <PerpsPositionsTable /> : null}
        {activeTab === "open-orders" ? <OpenOrders /> : null}
        {activeTab === "trade-history" ? <PerpsTradeHistory /> : null}
      </div>
    </div>
  );
};

type PerpsPositionRow = {
  pairId: string;
  symbol: string;
  size: string;
  entryPrice: string;
  currentPrice: number;
  pnl: number;
  estLiquidationPrice: number | null;
  conditionalOrderAbove?: ConditionalOrder;
  conditionalOrderBelow?: ConditionalOrder;
};

const PerpsPositionsTable: React.FC = () => {
  const { showModal, settings } = useApp();
  const { coins } = useConfig();
  const { formatNumberOptions } = settings;
  const { onChangePairId } = useProTrade();

  const userState = perpsUserStateStore((s) => s.userState);
  const extendedPositions = perpsUserStateExtendedStore((s) => s.positions);
  const equity = perpsUserStateExtendedStore((s) => s.equity);
  const perpsStatsByPairId = allPerpsPairStatsStore((s) => s.perpsPairStatsByPairId);

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
      const coinSymbol = coins.byDenom[baseDenom]?.symbol ?? baseSymbol.toUpperCase();
      const markPrice = Number(perpsStatsByPairId[pairId]?.currentPrice ?? pos.entryPrice);
      const size = Number(pos.size);
      const pnl = size * (markPrice - Number(pos.entryPrice));

      const backendLiqPrice = extendedPositions[pairId]?.liquidationPrice;
      const estLiquidationPrice = backendLiqPrice != null ? Number(backendLiqPrice) : null;

      result.push({
        pairId,
        symbol: coinSymbol,
        size: pos.size,
        entryPrice: pos.entryPrice,
        currentPrice: markPrice,
        pnl,
        estLiquidationPrice,
        conditionalOrderAbove: pos.conditionalOrderAbove,
        conditionalOrderBelow: pos.conditionalOrderBelow,
      });
    }
    return result;
  }, [userState, extendedPositions, perpsStatsByPairId, symbolToDenom, coins.byDenom]);

  const columns: TableColumn<PerpsPositionRow> = useMemo(
    () => [
      {
        header: m["dex.protrade.positions.pair"](),
        cell: ({ row }) => {
          const label = row.original.pairId
            .replace("perp/", "")
            .replace(/usd$/i, "/USD")
            .toUpperCase();
          return <Cell.Text text={label} className="diatype-xs-medium" />;
        },
      },
      {
        header: m["dex.protrade.positions.side"](),
        cell: ({ row }) => {
          const isLong = Number(row.original.size) > 0;
          return (
            <Cell.Text
              text={
                isLong ? m["dex.protrade.positions.long"]() : m["dex.protrade.positions.short"]()
              }
              className={isLong ? "text-utility-success-600" : "text-utility-error-600"}
            />
          );
        },
      },
      {
        header: m["dex.protrade.positions.size"](),
        cell: ({ row }) => {
          const absSize = Math.abs(Number(row.original.size)).toString();
          return (
            <Cell.Text
              text={
                <>
                  <FormattedNumber number={absSize} formatOptions={formatNumberOptions} as="span" />{" "}
                  {row.original.symbol}
                </>
              }
            />
          );
        },
      },
      {
        id: "positionValue",
        header: () => (
          <span className="block text-right">{m["dex.protrade.positions.positionValue"]()}</span>
        ),
        cell: ({ row }) => {
          const notional = Decimal(row.original.size)
            .abs()
            .times(Decimal(row.original.entryPrice))
            .toFixed();
          return (
            <Cell.Text
              className="text-right"
              text={
                <FormattedNumber number={notional} formatOptions={{ currency: "USD" }} as="span" />
              }
            />
          );
        },
      },
      {
        id: "entryPrice",
        header: () => (
          <span className="block text-right">{m["dex.protrade.positions.entryPrice"]()}</span>
        ),
        cell: ({ row }) => (
          <Cell.Text
            className="text-right"
            text={<FormattedNumber number={row.original.entryPrice} as="span" />}
          />
        ),
      },
      {
        id: "markPrice",
        header: () => (
          <span className="block text-right">{m["dex.protrade.positions.markPrice"]()}</span>
        ),
        cell: ({ row }) => (
          <Cell.Text
            className="text-right"
            text={<FormattedNumber number={row.original.currentPrice.toString()} as="span" />}
          />
        ),
      },
      {
        id: "pnl",
        header: () => <span className="block text-right">{m["dex.protrade.positions.pnl"]()}</span>,
        cell: ({ row }) => {
          const isPositive = row.original.pnl >= 0;
          return (
            <Cell.Text
              text={
                <span className="inline-flex items-center gap-1">
                  <span className="min-w-[7rem]">
                    {isPositive ? "+" : ""}
                    <FormattedNumber
                      number={row.original.pnl.toString()}
                      formatOptions={{ currency: "USD" }}
                      as="span"
                    />
                  </span>
                  <Button
                    variant="link"
                    size="xs"
                    className="p-0 h-fit m-0 overflow-visible"
                    onClick={(e) => {
                      e.stopPropagation();
                      showModal(Modals.PnlShare, {
                        pairId: row.original.pairId,
                        symbol: row.original.symbol,
                        size: row.original.size,
                        entryPrice: row.original.entryPrice,
                        currentPrice: row.original.currentPrice,
                        pnl: row.original.pnl,
                        equity,
                      });
                    }}
                  >
                    <IconShareNodes className="w-4 h-4" />
                  </Button>
                </span>
              }
              className={`text-right ${isPositive ? "text-utility-success-600" : "text-utility-error-600"}`}
            />
          );
        },
      },
      {
        id: "liqPrice",
        header: () => (
          <span className="block text-right">{m["dex.protrade.positions.liqPrice"]()}</span>
        ),
        cell: ({ row }) => (
          <Cell.Text
            className="text-right"
            text={
              row.original.estLiquidationPrice != null ? (
                <FormattedNumber
                  number={row.original.estLiquidationPrice.toString()}
                  formatOptions={{ currency: "USD" }}
                  as="span"
                />
              ) : (
                "N/A"
              )
            }
          />
        ),
      },
      {
        id: "tpsl",
        header: () => (
          <span className="block text-right">{m["dex.protrade.positions.tpsl"]()}</span>
        ),
        cell: ({ row }: { row: { original: PerpsPositionRow } }) => {
          const { size, conditionalOrderAbove, conditionalOrderBelow } = row.original;
          const isLong = Number(size) > 0;
          const tp = isLong ? conditionalOrderAbove : conditionalOrderBelow;
          const sl = isLong ? conditionalOrderBelow : conditionalOrderAbove;
          const tpDisplay = tp ? formatNumber(tp.triggerPrice, formatNumberOptions) : "--";
          const slDisplay = sl ? formatNumber(sl.triggerPrice, formatNumberOptions) : "--";
          return (
            <div className="flex items-center gap-1 justify-end">
              <Cell.Text text={`${tpDisplay}/${slDisplay}`} />
              <button
                type="button"
                className="text-ink-tertiary-500 hover:text-ink-secondary-700 diatype-xs-regular underline ml-1"
                onClick={(e) => {
                  e.stopPropagation();
                  const hasAny = conditionalOrderAbove || conditionalOrderBelow;
                  if (hasAny) {
                    showModal(Modals.ProSwapEditedSL, {
                      pairId: row.original.pairId,
                      symbol: row.original.symbol,
                      entryPrice: row.original.entryPrice,
                      markPrice: row.original.currentPrice.toString(),
                      size: row.original.size,
                      conditionalOrderAbove,
                      conditionalOrderBelow,
                    });
                  } else {
                    showModal(Modals.ProSwapEditTPSL, {
                      pairId: row.original.pairId,
                      symbol: row.original.symbol,
                      entryPrice: row.original.entryPrice,
                      markPrice: row.original.currentPrice.toString(),
                      size: row.original.size,
                      conditionalOrderAbove,
                      conditionalOrderBelow,
                    });
                  }
                }}
              >
                {m["dex.protrade.positions.edit"]()}
              </button>
            </div>
          );
        },
      },
      {
        id: "close-position",
        header: () => <Cell.Text text="" />,
        cell: ({ row }) => (
          <div onClick={(e) => e.stopPropagation()}>
            <Cell.Action
              action={() =>
                showModal(Modals.PerpsClosePosition, {
                  pairId: row.original.pairId,
                  size: row.original.size,
                  pnl: row.original.pnl,
                })
              }
              label={m["dex.protrade.positions.close"]()}
              classNames={{
                cell: "items-end",
                button: "!exposure-xs-italic m-0 p-0 px-1 h-fit",
              }}
            />
          </div>
        ),
      },
    ],
    [formatNumberOptions, showModal],
  );

  const handleRowClick = useCallback(
    (row: { original: PerpsPositionRow }) => {
      onChangePairId(`${row.original.symbol}-USD`);
    },
    [onChangePairId],
  );

  return (
    <Table
      data={rows}
      columns={columns}
      style="simple"
      onRowClick={handleRowClick}
      classNames={{
        row: "h-fit",
        header: "pt-0",
        base: "pb-[1.5rem] max-h-[18rem] overflow-y-scroll",
        cell: twMerge("diatype-xs-regular py-1", {
          "group-hover:bg-transparent": !rows.length,
        }),
      }}
      emptyComponent={
        <EmptyPlaceholder
          component={m["dex.protrade.positions.noOpenPositions"]()}
          className="h-[3.5rem]"
        />
      }
    />
  );
};

type OpenOrder = {
  id: string;
  pairDisplay: string;
  side: "buy" | "sell";
  type: "limit";
  rawPrice: string;
  size: string;
  reduceOnly: boolean;
};

const OpenOrders: React.FC = () => {
  const { showModal } = useApp();

  const { baseCoin } = useTradeCoins();

  const [showAllPairs, setShowAllPairs] = useState(true);

  const perpsOrders = perpsOrdersByUserStore((s) => s.orders);

  const currentPerpsPairId = useMemo(() => {
    const base = baseCoin.symbol?.toLowerCase() ?? "";
    return `perp/${base}usd`;
  }, [baseCoin.symbol]);

  const rows = useMemo(() => {
    const result: OpenOrder[] = [];
    if (!perpsOrders) return result;

    const allPerpsOrders = Object.entries(perpsOrders);
    const filtered = showAllPairs
      ? allPerpsOrders
      : allPerpsOrders.filter(([, o]) => o.pairId === currentPerpsPairId);

    for (const [orderId, order] of filtered) {
      const label = order.pairId.replace("perp/", "").replace(/usd$/i, "/USD").toUpperCase();
      const isLong = Number(order.size) > 0;

      result.push({
        id: orderId,
        pairDisplay: label,
        side: isLong ? "buy" : "sell",
        type: "limit",
        rawPrice: order.limitPrice,
        size: Math.abs(Number(order.size)).toString(),
        reduceOnly: order.reduceOnly,
      });
    }

    return result;
  }, [perpsOrders, showAllPairs, currentPerpsPairId]);

  const columns: TableColumn<OpenOrder> = [
    {
      header: m["dex.protrade.orders.market"](),
      cell: () => <Badge text={m["dex.protrade.orders.perp"]()} color="green" size="s" />,
    },
    {
      header: m["dex.protrade.orders.pair"](),
      cell: ({ row }) => (
        <Cell.Text text={row.original.pairDisplay} className="diatype-xs-medium" />
      ),
    },
    {
      header: m["dex.protrade.orders.side"](),
      cell: ({ row }) => {
        const isBuy = row.original.side === "buy";
        return (
          <Cell.Text
            text={isBuy ? m["dex.protrade.orders.long"]() : m["dex.protrade.orders.short"]()}
            className={isBuy ? "text-utility-success-600" : "text-utility-error-600"}
          />
        );
      },
    },
    {
      header: m["dex.protrade.orders.type"](),
      cell: () => <Cell.Text text={m["dex.protrade.orders.limit"]()} />,
    },
    {
      header: m["dex.protrade.orders.price"](),
      cell: ({ row }) => (
        <Cell.Text
          text={
            <FormattedNumber
              number={row.original.rawPrice}
              formatOptions={{ currency: "USD", maxFractionDigits: 6 }}
              as="span"
            />
          }
        />
      ),
    },
    {
      header: m["dex.protrade.orders.size"](),
      cell: ({ row }) => (
        <Cell.Number formatOptions={{ maxFractionDigits: 6 }} value={row.original.size} />
      ),
    },
    {
      header: m["dex.protrade.orders.orderValue"](),
      cell: ({ row }) => {
        const tradeValue = Decimal(row.original.size)
          .times(Decimal(row.original.rawPrice))
          .toFixed();
        return (
          <Cell.Text
            text={
              <FormattedNumber
                number={tradeValue}
                formatOptions={{ currency: "USD", maxFractionDigits: 6 }}
                as="span"
              />
            }
          />
        );
      },
    },
    {
      header: m["dex.protrade.orders.filled"](),
      cell: () => <Cell.Text text="-" className="text-ink-tertiary-500" />,
    },
    {
      header: m["dex.protrade.orders.reduceOnly"](),
      cell: ({ row }) =>
        row.original.reduceOnly ? (
          <Badge text="Yes" color="warning" size="s" />
        ) : (
          <Cell.Text text="-" className="text-ink-tertiary-500" />
        ),
    },
    {
      header: m["dex.protrade.orders.tpsl"](),
      cell: () => <Cell.Text text="--/--" className="text-ink-tertiary-500" />,
    },
    {
      id: "cancel-order",
      header: () => (
        <div className="flex items-center justify-end gap-2">
          <label className="flex items-center gap-1.5 cursor-pointer diatype-xs-regular text-ink-tertiary-500">
            <input
              type="checkbox"
              checked={showAllPairs}
              onChange={(e) => setShowAllPairs(e.target.checked)}
              className="accent-primitives-red-light-500 w-3 h-3"
            />
            {m["dex.protrade.orders.showAllPairs"]()}
          </label>
          <Cell.Action
            isDisabled={!rows.length}
            action={() => showModal(Modals.PerpsCloseAll, {})}
            label={m["common.cancelAll"]()}
            classNames={{
              cell: "items-end diatype-xs-regular",
              button: "!exposure-xs-italic m-0 p-0 px-1 h-fit",
            }}
          />
        </div>
      ),
      cell: ({ row }) => (
        <Cell.Action
          action={() => showModal(Modals.PerpsCloseOrder, { orderId: row.original.id })}
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
      data={rows}
      columns={columns}
      style="simple"
      classNames={{
        row: "h-fit",
        header: "pt-0",
        base: "pb-[1.5rem] max-h-[18rem] overflow-y-scroll",
        cell: twMerge("diatype-xs-regular py-1", {
          "group-hover:bg-transparent": !rows.length,
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

export const ProTrade = Object.assign(ProTradeContainer, {
  Header: ProTradeHeader,
  Chart: ProTradeChart,
  History: ProTradeHistory,
  OrderBook: ProTradeOverview,
  TradeMenu: ProTradeMenu,
});
