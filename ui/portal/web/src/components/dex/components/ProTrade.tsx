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
  useOrderBookState,
  useLivePerpsTradesState,
  usePerpsUserState,
  usePerpsUserStateExtended,
  useOrdersByUser,
  usePerpsOrdersByUser,
  perpsOrdersByUserStore,
  perpsUserStateStore,
  perpsUserStateExtendedStore,
  TradePairStore,
  tradeInfoStore,
  useTradeCoins,
  useLiveSpotTradesState,
  usePerpsPairState,
  useAllPairStats,
  useAllPerpsPairStats,
  allPerpsPairStatsStore,
  useCurrentPrice,
  useOraclePrices,
} from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { createPortal } from "react-dom";
import { Decimal, formatNumber } from "@left-curve/dango/utils";

import { Cell, FormattedNumber, IconLink, Table, Tabs } from "@left-curve/applets-kit";
import { CountBadge } from "../../foundation/CountBadge";
import { EmptyPlaceholder } from "../../foundation/EmptyPlaceholder";
import { OrderBookOverview } from "./OrderBookOverview";
import { TradeButtons } from "./TradeButtons";
import { TradeMenu } from "./TradeMenu";
import { TradeHeader } from "./TradeHeader";
import { ErrorBoundary } from "react-error-boundary";
import { SpotTradeHistory, PerpsTradeHistory } from "./TradeHistory";

import type { PropsWithChildren } from "react";
import type { TableColumn } from "@left-curve/applets-kit";
import type { ConditionalOrder, OrderId, PairId } from "@left-curve/dango/types";

const [ProTradeProvider, useProTrade] = createContext<{
  controllers: ReturnType<typeof useInputs>;
  onChangePairId: (pairSymbols: string, type: "spot" | "perps") => void;
}>({
  name: "ProTradeContext",
});

export { useProTrade };

const TradeDocumentTitle: React.FC = () => {
  const mode = TradePairStore((s) => s.mode);
  const pairId = TradePairStore((s) => s.pairId);
  const { baseCoin, quoteCoin } = useTradeCoins();
  const { currentPrice } = useCurrentPrice();

  useEffect(() => {
    const previousTitle = document.title;
    const symbol =
      mode === "perps" ? `${baseCoin.symbol}-USD` : `${baseCoin.symbol}-${quoteCoin.symbol}`;

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
  }, [mode, pairId, baseCoin.symbol, quoteCoin.symbol, currentPrice]);

  return null;
};

const TradeSubscriptions: React.FC = () => {
  const mode = TradePairStore((s) => s.mode);

  // Spot subscriptions disabled — winding down spot trading
  useOrderBookState({ subscribe: false });

  useLivePerpsTradesState({ subscribe: mode === "perps" });
  useLiveSpotTradesState({ subscribe: false });

  usePerpsUserState({ subscribe: mode === "perps" });
  usePerpsUserStateExtended({ subscribe: mode === "perps" });
  usePerpsPairState({ subscribe: mode === "perps" });
  usePerpsOrdersByUser({ subscribe: mode === "perps" });

  useOraclePrices({ subscribe: true });

  useAllPairStats();
  useAllPerpsPairStats();

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
  type = "perps",
  children,
}) => {
  const controllers = useInputs();

  useEffect(() => {
    TradePairStore.getState().setPair(pairId, type);
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
  const mode = TradePairStore((s) => s.mode);
  const { isLg } = useMediaQuery();

  const { baseCoin, quoteCoin } = useTradeCoins();

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

  const Chart = (
    <Suspense fallback={<Spinner color="pink" size="md" />}>
      <div className="flex w-full lg:min-h-[32.875rem] h-full" id="chart-container">
        <ErrorBoundary fallback={<div className="p-4">Chart Engine</div>}>
          <TradingView
            coins={{ base: baseCoin, quote: quoteCoin }}
            orders={ordersByPair}
            mode={mode}
          />
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
  const mode = TradePairStore((s) => s.mode);
  const defaultTab: BottomTab = mode === "perps" ? "positions" : "open-orders";
  const [activeTab, setActiveTab] = useState<BottomTab>(defaultTab);

  const userState = perpsUserStateStore((s) => s.userState);
  const spotOrders = useOrdersByUser({ enabled: mode === "spot", initialData: [] });
  const perpsOrders = perpsOrdersByUserStore((s) => s.orders);

  const positionsCount = Object.keys(userState?.positions ?? {}).length;
  const openOrdersCount =
    mode === "spot" ? (spotOrders.data?.length ?? 0) : Object.keys(perpsOrders ?? {}).length;

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
          {mode === "perps" ? (
            <Tab title="positions">
              <span className="flex items-center gap-1">
                {m["dex.protrade.positions.title"]()}
                <CountBadge count={positionsCount} />
              </span>
            </Tab>
          ) : null}
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
        {activeTab === "positions" && mode === "perps" ? <PerpsPositionsTable /> : null}
        {activeTab === "open-orders" ? <UnifiedOpenOrders /> : null}
        {activeTab === "trade-history" ? <ProTradeOrdersHistory /> : null}
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
            text={
              <FormattedNumber
                number={row.original.entryPrice}
                as="span"
              />
            }
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
            text={
              <FormattedNumber
                number={row.original.currentPrice.toString()}
                as="span"
              />
            }
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
                  <button
                    type="button"
                    className="text-ink-tertiary-500 hover:text-ink-secondary-700 cursor-pointer"
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
                    <IconLink className="w-4 h-4" />
                  </button>
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
      onChangePairId(`${row.original.symbol}-USD`, "perps");
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

type UnifiedOrder = {
  id: string;
  market: "spot" | "perps";
  pairDisplay: string;
  side: "buy" | "sell";
  type: "limit";
  price: string;
  rawPrice: string;
  size: string;
  filled: string | null;
  reduceOnly: boolean;
  rawSpotOrderId?: OrderId;
  rawPerpsOrderId?: string;
};

const UnifiedOpenOrders: React.FC = () => {
  const { showModal, settings } = useApp();
  const { coins } = useConfig();
  const mode = TradePairStore((s) => s.mode);

  const { formatNumberOptions } = settings;
  const { baseCoin } = useTradeCoins();

  const [showAllPairs, setShowAllPairs] = useState(true);

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
        const spotPrice = Decimal(order.price)
          .times(Decimal(10).pow(baseDecimals - quoteDecimals))
          .toFixed();

        rows.push({
          id: order.id,
          market: "spot",
          pairDisplay: `${baseSymbol}/${quoteSymbol}`,
          side: order.direction === "bid" ? "buy" : "sell",
          type: "limit",
          price: formatNumber(spotPrice, formatNumberOptions),
          rawPrice: spotPrice,
          size: originalSize,
          filled: filledQty,
          reduceOnly: false,
          rawSpotOrderId: order.id,
        });
      }
    }

    if (mode === "perps" && perpsOrders) {
      const allPerpsOrders = Object.entries(perpsOrders);
      const filtered = showAllPairs
        ? allPerpsOrders
        : allPerpsOrders.filter(([, o]) => o.pairId === currentPerpsPairId);

      for (const [orderId, order] of filtered) {
        const label = order.pairId.replace("perp/", "").replace(/usd$/i, "/USD").toUpperCase();
        const isLong = Number(order.size) > 0;

        rows.push({
          id: orderId,
          market: "perps",
          pairDisplay: label,
          side: isLong ? "buy" : "sell",
          type: "limit",
          price: `$${formatNumber(order.limitPrice, formatNumberOptions)}`,
          rawPrice: order.limitPrice,
          size: Math.abs(Number(order.size)).toString(),
          filled: null,
          reduceOnly: order.reduceOnly,
          rawPerpsOrderId: orderId,
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
      header: m["dex.protrade.orders.market"](),
      cell: ({ row }) => (
        <Badge
          text={
            row.original.market === "spot"
              ? m["dex.protrade.orders.spot"]()
              : m["dex.protrade.orders.perp"]()
          }
          color={row.original.market === "spot" ? "blue" : "green"}
          size="s"
        />
      ),
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
            text={
              row.original.market === "perps"
                ? isBuy
                  ? m["dex.protrade.orders.long"]()
                  : m["dex.protrade.orders.short"]()
                : isBuy
                  ? m["dex.protrade.orders.buy"]()
                  : m["dex.protrade.orders.sell"]()
            }
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
      cell: ({ row }) => <Cell.Text text={row.original.price} />,
    },
    {
      header: m["dex.protrade.orders.size"](),
      cell: ({ row }) => (
        <Cell.Number formatOptions={formatNumberOptions} value={row.original.size} />
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
              <FormattedNumber number={tradeValue} formatOptions={{ currency: "USD" }} as="span" />
            }
          />
        );
      },
    },
    {
      header: m["dex.protrade.orders.filled"](),
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
            header: m["dex.protrade.orders.reduceOnly"](),
            cell: ({ row }: { row: { original: UnifiedOrder } }) =>
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
        ]
      : []),
    {
      id: "cancel-order",
      header: () => (
        <div className="flex items-center justify-end gap-2">
          {mode === "perps" && (
            <label className="flex items-center gap-1.5 cursor-pointer diatype-xs-regular text-ink-tertiary-500">
              <input
                type="checkbox"
                checked={showAllPairs}
                onChange={(e) => setShowAllPairs(e.target.checked)}
                className="accent-primitives-red-light-500 w-3 h-3"
              />
              {m["dex.protrade.orders.showAllPairs"]()}
            </label>
          )}
          <Cell.Action
            isDisabled={!unifiedRows.length}
            action={() => {
              if (mode === "spot") {
                const ids = unifiedRows
                  .filter((o) => o.rawSpotOrderId)
                  .map((o) => o.rawSpotOrderId!);
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
        </div>
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
    <Table
      data={unifiedRows}
      columns={columns}
      style="simple"
      classNames={{
        row: "h-fit",
        header: "pt-0",
        base: "pb-[1.5rem] max-h-[18rem] overflow-y-scroll",
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
  );
};

const ProTradeOrdersHistory: React.FC = () => {
  const mode = TradePairStore((s) => s.mode);
  return mode === "perps" ? <PerpsTradeHistory /> : <SpotTradeHistory />;
};

export const ProTrade = Object.assign(ProTradeContainer, {
  Header: ProTradeHeader,
  Chart: ProTradeChart,
  History: ProTradeHistory,
  OrderBook: ProTradeOverview,
  TradeMenu: ProTradeMenu,
});
