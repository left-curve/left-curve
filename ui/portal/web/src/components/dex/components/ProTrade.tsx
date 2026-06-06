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
  getPerpsPairIdFromPairId,
  useLivePerpsTrades,
  useConfig,
  useOraclePrices,
  usePerpsUserState,
  usePerpsUserStateExtended,
  usePerpsOrdersByUser,
  useAllPerpsPairStats,
  useCurrentPrice,
  usePerpsPairState,
  usePerpsState,
  useTradePairCoins,
  useAccount,
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
import { reportStoreError } from "../../../app.sentry";
import { OrderBookOverview } from "./OrderBookOverview";
import { TradeButtons } from "./TradeButtons";
import { TradeMenu } from "./TradeMenu";
import { TradeHeader } from "./TradeHeader";
import { ErrorBoundary } from "react-error-boundary";
import { PerpsTradeHistory } from "./TradeHistory";

import type { PropsWithChildren } from "react";
import type { TableColumn } from "@left-curve/applets-kit";
import type { ConditionalOrder, PairId } from "@left-curve/types";

const PRO_TRADE_LIVE_RESOURCE_ERROR_TOAST_ID = "protrade-live-resource-error";

type LiveResourceErrorSnapshot = {
  status: "idle" | "connecting" | "ready" | "error";
  error: Error | null;
};

type LiveResourceErrorSource = {
  name: string;
  error: Error | null;
};

function selectLiveResourceError(snapshot: LiveResourceErrorSnapshot) {
  return snapshot.status === "error" ? snapshot.error : null;
}

const [ProTradeProvider, useProTrade] = createContext<{
  controllers: ReturnType<typeof useInputs>;
  pairId: PairId;
  perpsPairId: string;
  action: "buy" | "sell";
  orderType: "limit" | "market";
  accountAddress?: string;
  onChangePairId: (pairSymbols: string) => void;
  onChangeAction: (action: "buy" | "sell") => void;
  onChangeOrderType: (orderType: "limit" | "market") => void;
}>({
  name: "ProTradeContext",
});

export { useProTrade };

const TradeDocumentTitle: React.FC = () => {
  const { pairId, perpsPairId } = useProTrade();
  const { baseCoin } = useTradePairCoins({ pairId });
  const { currentPrice } = useCurrentPrice({ perpsPairId });

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

const ProTradeErrorFallback: React.FC = () => {
  return (
    <div className="flex h-full min-h-[24rem] w-full items-center justify-center bg-surface-primary-rice p-6 text-center diatype-sm-medium text-ink-tertiary-500">
      Market data unavailable
    </div>
  );
};

const ProTradeLiveResourceErrors: React.FC = () => {
  const { toast } = useApp();
  const { accountAddress, perpsPairId } = useProTrade();

  const allPairStatsError = useAllPerpsPairStats(selectLiveResourceError);
  const liveTradesError = useLivePerpsTrades(selectLiveResourceError, { perpsPairId });
  const oraclePricesError = useOraclePrices(selectLiveResourceError);
  const pairStateError = usePerpsPairState(selectLiveResourceError, { perpsPairId });
  const perpsStateError = usePerpsState(selectLiveResourceError);
  const userStateError = usePerpsUserState(selectLiveResourceError, { accountAddress });
  const userStateExtendedError = usePerpsUserStateExtended(selectLiveResourceError, {
    accountAddress,
  });
  const userOrdersError = usePerpsOrdersByUser(selectLiveResourceError, { accountAddress });

  const activeError = useMemo<LiveResourceErrorSource | null>(() => {
    const errors: LiveResourceErrorSource[] = [
      { name: "market stats", error: allPairStatsError },
      { name: "live trades", error: liveTradesError },
      { name: "oracle prices", error: oraclePricesError },
      { name: "pair state", error: pairStateError },
      { name: "perps state", error: perpsStateError },
      { name: "account state", error: userStateError },
      { name: "account margin", error: userStateExtendedError },
      { name: "open orders", error: userOrdersError },
    ];

    return errors.find(({ error }) => error) ?? null;
  }, [
    allPairStatsError,
    liveTradesError,
    oraclePricesError,
    pairStateError,
    perpsStateError,
    userStateError,
    userStateExtendedError,
    userOrdersError,
  ]);

  useEffect(() => {
    if (!activeError?.error) {
      toast.dismiss(PRO_TRADE_LIVE_RESOURCE_ERROR_TOAST_ID);
      return;
    }

    reportStoreError(activeError.error);
    toast.dismiss(PRO_TRADE_LIVE_RESOURCE_ERROR_TOAST_ID);
    toast.error(
      {
        title: m["common.error"](),
        description: `Live ${activeError.name} error: ${activeError.error.message}`,
      },
      {
        id: PRO_TRADE_LIVE_RESOURCE_ERROR_TOAST_ID,
        duration: Number.POSITIVE_INFINITY,
      },
    );
  }, [activeError, toast]);

  useEffect(() => {
    return () => toast.dismiss(PRO_TRADE_LIVE_RESOURCE_ERROR_TOAST_ID);
  }, [toast]);

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
  const { coins } = useConfig();
  const { account } = useAccount();
  const perpsPairId = useMemo(() => getPerpsPairIdFromPairId(pairId, coins), [pairId, coins]);
  const contextValue = useMemo(
    () => ({
      controllers,
      pairId,
      perpsPairId,
      action,
      orderType,
      accountAddress: account?.address,
      onChangePairId,
      onChangeAction,
      onChangeOrderType,
    }),
    [
      controllers,
      pairId,
      perpsPairId,
      action,
      orderType,
      account?.address,
      onChangePairId,
      onChangeAction,
      onChangeOrderType,
    ],
  );

  if (!pairId.baseDenom || !pairId.quoteDenom || !perpsPairId) return null;

  return (
    <ProTradeProvider value={contextValue}>
      <ErrorBoundary
        fallback={<ProTradeErrorFallback />}
        resetKeys={[pairId.baseDenom, pairId.quoteDenom, perpsPairId]}
      >
        <ProTradeLiveResourceErrors />
        <TradeDocumentTitle />
        {children}
      </ErrorBoundary>
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
  const { pairId, perpsPairId, accountAddress } = useProTrade();

  const { baseCoin, quoteCoin } = useTradePairCoins({ pairId });

  const mobileContainer = usePortalTarget("#chart-container-mobile");

  const Chart = (
    <Suspense fallback={<Spinner color="pink" size="md" fullContainer />}>
      <div className="flex w-full lg:min-h-[32.875rem] h-full" id="chart-container">
        <ErrorBoundary fallback={<div className="p-4">Chart Engine</div>}>
          <TradingView
            coins={{ base: baseCoin, quote: quoteCoin }}
            perpsPairId={perpsPairId}
            accountAddress={accountAddress}
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
  const [activeTab, setActiveTab] = useState<BottomTab>("positions");
  const { accountAddress } = useProTrade();

  const userState = usePerpsUserState((s) => s.userState, { accountAddress });
  const perpsOrders = usePerpsOrdersByUser((s) => s.orders, { accountAddress });

  const positionsCount = Object.keys(userState?.positions ?? {}).length;
  const openOrdersCount = Object.keys(perpsOrders ?? {}).length;

  return (
    <div className="flex-1 max-w-[100vw] lg:max-w-none p-4 bg-surface-primary-rice flex flex-col gap-2 shadow-account-card pb-20 lg:pb-5 z-10">
      <div className="relative flex items-center justify-between">
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
      {activeTab === "trade-history" ? (
        <div className="w-full h-full relative">
          <PerpsTradeHistory />
        </div>
      ) : (
        <div className="w-full h-full relative">
          {activeTab === "positions" ? <PerpsPositionsTable /> : null}
          {activeTab === "open-orders" ? <OpenOrders /> : null}
        </div>
      )}
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
  const { accountAddress, onChangePairId } = useProTrade();

  const userState = usePerpsUserState((s) => s.userState, { accountAddress });
  const extendedPositions = usePerpsUserStateExtended((s) => s.positions, { accountAddress });
  const equity = usePerpsUserStateExtended((s) => s.equity, { accountAddress });
  const perpsStatsByPairId = useAllPerpsPairStats((s) => s.perpsPairStatsByPairId);

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
                        mode: "position",
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
  const { accountAddress, perpsPairId } = useProTrade();

  const [showAllPairs, setShowAllPairs] = useState(true);

  const perpsOrders = usePerpsOrdersByUser((s) => s.orders, { accountAddress });

  const rows = useMemo(() => {
    const result: OpenOrder[] = [];
    if (!perpsOrders) return result;

    const allPerpsOrders = Object.entries(perpsOrders);
    const filtered = showAllPairs
      ? allPerpsOrders
      : allPerpsOrders.filter(([, o]) => o.pairId === perpsPairId);

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
  }, [perpsOrders, showAllPairs, perpsPairId]);

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
