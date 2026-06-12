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
  usePerpsUserState,
  usePerpsUserStateExtended,
  usePerpsOrdersByUser,
  useCurrentPrice,
  useAccount,
  useAllPerpsPairStats,
} from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { MarketPair } from "@left-curve/foundation/market-pair";
import { createPortal } from "react-dom";
import { Decimal, formatNumber, shallowEqual } from "@left-curve/utils";

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
import type { ConditionalOrder } from "@left-curve/types";

const USD_MAX_FRACTION_DIGITS_FORMAT_OPTIONS = { currency: "USD", maxFractionDigits: 6 } as const;
const MAX_FRACTION_DIGITS_FORMAT_OPTIONS = { maxFractionDigits: 6 } as const;

const [ProTradeProvider, useProTrade] = createContext<{
  pair: MarketPair;
  action: "buy" | "sell";
  orderType: "limit" | "market";
  accountAddress?: string;
  onChangeTicker: (ticker: string) => void;
  onChangeAction: (action: "buy" | "sell") => void;
  onChangeOrderType: (orderType: "limit" | "market") => void;
}>({
  name: "ProTradeContext",
});

const [ProTradeFormProvider, useProTradeForm] = createContext<ReturnType<typeof useInputs>>({
  name: "ProTradeFormContext",
});

const [ProTradeFormActionsProvider, useProTradeFormActions] = createContext<{
  setValue: ReturnType<typeof useInputs>["setValue"];
}>({
  name: "ProTradeFormActionsContext",
});

export { useProTrade };

const TradeDocumentTitle: React.FC = () => {
  const { pair } = useProTrade();
  const { currentPrice } = useCurrentPrice({ pairId: pair.id });

  useEffect(() => {
    const previousTitle = document.title;

    if (currentPrice) {
      const priceNum = Number(currentPrice);
      const formatted = Number.isFinite(priceNum)
        ? priceNum.toLocaleString(undefined, {
            minimumFractionDigits: 2,
            maximumFractionDigits: priceNum < 1 ? 6 : 2,
          })
        : currentPrice;
      document.title = `${formatted} | ${pair.ticker} | Dango`;
    } else {
      document.title = `${pair.ticker} · Dango`;
    }

    return () => {
      document.title = previousTitle;
    };
  }, [pair, currentPrice]);

  return null;
};

const ProTradeErrorFallback: React.FC = () => {
  return (
    <div className="flex h-full min-h-[24rem] w-full items-center justify-center bg-surface-primary-rice p-6 text-center diatype-sm-medium text-ink-tertiary-500">
      Market data unavailable
    </div>
  );
};

type ProTradeProps = {
  action: "buy" | "sell";
  onChangeAction: (action: "buy" | "sell") => void;
  pair: MarketPair;
  onChangeTicker: (ticker: string) => void;
  orderType: "limit" | "market";
  onChangeOrderType: (orderType: "limit" | "market") => void;
};

const ProTradeContainer: React.FC<PropsWithChildren<ProTradeProps>> = ({
  pair,
  action,
  onChangeTicker,
  onChangeAction,
  orderType,
  onChangeOrderType,
  children,
}) => {
  const { account } = useAccount();
  const controllers = useInputs();
  const formActionsContextValue = useMemo(
    () => ({ setValue: controllers.setValue }),
    [controllers.setValue],
  );
  const contextValue = useMemo(
    () => ({
      pair,
      action,
      orderType,
      accountAddress: account?.address,
      onChangeTicker,
      onChangeAction,
      onChangeOrderType,
    }),
    [pair, action, orderType, account?.address, onChangeTicker, onChangeAction, onChangeOrderType],
  );

  return (
    <ProTradeProvider value={contextValue}>
      <ProTradeFormProvider value={controllers}>
        <ProTradeFormActionsProvider value={formActionsContextValue}>
          <ErrorBoundary fallback={<ProTradeErrorFallback />} resetKeys={[pair.id]}>
            <TradeDocumentTitle />
            {children}
          </ErrorBoundary>
        </ProTradeFormActionsProvider>
      </ProTradeFormProvider>
    </ProTradeProvider>
  );
};

const ProTradeHeader: React.FC = () => {
  return <TradeHeader />;
};

const ProTradeOverview: React.FC = () => {
  const { setValue } = useProTradeFormActions();
  const handleSelectPrice = useCallback((price: string) => setValue("price", price), [setValue]);

  return <OrderBookOverview onSelectPrice={handleSelectPrice} />;
};

const TradingView = lazy(() =>
  import("./TradingView").then(({ TradingView }) => ({ default: TradingView })),
);

const ProTradeChart: React.FC = () => {
  const { isLg } = useMediaQuery();
  const { pair, accountAddress } = useProTrade();

  const mobileContainer = usePortalTarget("#chart-container-mobile");

  const Chart = (
    <Suspense fallback={<Spinner color="pink" size="md" fullContainer />}>
      <div className="flex w-full lg:min-h-[32.875rem] h-full" id="chart-container">
        <ErrorBoundary fallback={<div className="p-4">Chart Engine</div>}>
          <TradingView pair={pair} accountAddress={accountAddress} />
        </ErrorBoundary>
      </div>
    </Suspense>
  );

  return <>{isLg ? Chart : mobileContainer ? createPortal(Chart, mobileContainer) : null}</>;
};

const ProTradeMenu: React.FC = () => {
  const { isLg } = useMediaQuery();
  const controllers = useProTradeForm();

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

  const positionsCount = usePerpsUserState(
    (s) => Object.keys(s.userState?.positions ?? {}).length,
    { accountAddress },
  );
  const openOrdersCount = usePerpsOrdersByUser((s) => Object.keys(s.orders ?? {}).length, {
    accountAddress,
  });

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
  pair: MarketPair;
  size: string;
  entryPrice: string;
  currentPrice: number;
  pnl: number;
  estLiquidationPrice: number | null;
  conditionalOrderAbove?: ConditionalOrder;
  conditionalOrderBelow?: ConditionalOrder;
};

const PerpsPositionsTable: React.FC = () => {
  const showModal = useApp((state) => state.showModal);
  const formatNumberOptions = useApp((state) => state.settings.formatNumberOptions);
  const { accountAddress, onChangeTicker } = useProTrade();

  const positions = usePerpsUserState((s) => s.userState?.positions ?? null, { accountAddress });
  const extendedState = usePerpsUserStateExtended(
    (s) => ({ positions: s.positions, equity: s.equity }),
    { accountAddress },
    shallowEqual,
  );
  const extendedPositions = extendedState.positions;
  const equity = extendedState.equity;
  const hasPositions = Object.keys(positions ?? {}).length > 0;
  const perpsStatsByPairId = useAllPerpsPairStats((s) => s.perpsPairStatsByPairId, {
    enabled: hasPositions,
  });

  const rows = useMemo(() => {
    const result: PerpsPositionRow[] = [];
    if (!positions) return result;

    for (const [pairId, pos] of Object.entries(positions)) {
      const pair = MarketPair.fromPairId(pairId);
      const markPrice = Number(perpsStatsByPairId[pairId]?.currentPrice ?? pos.entryPrice);
      const size = Number(pos.size);
      const pnl = size * (markPrice - Number(pos.entryPrice));

      const backendLiqPrice = extendedPositions[pairId]?.liquidationPrice;
      const estLiquidationPrice = backendLiqPrice != null ? Number(backendLiqPrice) : null;

      result.push({
        pair,
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
  }, [positions, extendedPositions, perpsStatsByPairId]);

  const columns: TableColumn<PerpsPositionRow> = useMemo(
    () => [
      {
        header: m["dex.protrade.positions.pair"](),
        cell: ({ row }) => {
          return <Cell.Text text={row.original.pair.ticker} className="diatype-xs-medium" />;
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
                  {row.original.pair.base.symbol}
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
                        pairId: row.original.pair.id,
                        symbol: row.original.pair.base.symbol,
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
                      pairId: row.original.pair.id,
                      symbol: row.original.pair.base.symbol,
                      entryPrice: row.original.entryPrice,
                      markPrice: row.original.currentPrice.toString(),
                      size: row.original.size,
                      conditionalOrderAbove,
                      conditionalOrderBelow,
                    });
                  } else {
                    showModal(Modals.ProSwapEditTPSL, {
                      pairId: row.original.pair.id,
                      symbol: row.original.pair.base.symbol,
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
                  pairId: row.original.pair.id,
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
      onChangeTicker(row.original.pair.ticker);
    },
    [onChangeTicker],
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
  ticker: string;
  side: "buy" | "sell";
  type: "limit";
  rawPrice: string;
  size: string;
  reduceOnly: boolean;
};

const OpenOrders: React.FC = () => {
  const showModal = useApp((state) => state.showModal);
  const { accountAddress, pair } = useProTrade();

  const [showAllPairs, setShowAllPairs] = useState(true);

  const perpsOrders = usePerpsOrdersByUser((s) => s.orders, { accountAddress });

  const rows = useMemo(() => {
    const result: OpenOrder[] = [];
    if (!perpsOrders) return result;

    const allPerpsOrders = Object.entries(perpsOrders);
    const filtered = showAllPairs
      ? allPerpsOrders
      : allPerpsOrders.filter(([, o]) => o.pairId === pair.id);

    for (const [orderId, order] of filtered) {
      const isLong = Number(order.size) > 0;

      result.push({
        id: orderId,
        ticker: MarketPair.fromPairId(order.pairId).ticker,
        side: isLong ? "buy" : "sell",
        type: "limit",
        rawPrice: order.limitPrice,
        size: Math.abs(Number(order.size)).toString(),
        reduceOnly: order.reduceOnly,
      });
    }

    return result;
  }, [perpsOrders, showAllPairs, pair.id]);

  const columns: TableColumn<OpenOrder> = [
    {
      header: m["dex.protrade.orders.market"](),
      cell: () => <Badge text={m["dex.protrade.orders.perp"]()} color="green" size="s" />,
    },
    {
      header: m["dex.protrade.orders.pair"](),
      cell: ({ row }) => <Cell.Text text={row.original.ticker} className="diatype-xs-medium" />,
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
              formatOptions={USD_MAX_FRACTION_DIGITS_FORMAT_OPTIONS}
              as="span"
            />
          }
        />
      ),
    },
    {
      header: m["dex.protrade.orders.size"](),
      cell: ({ row }) => (
        <Cell.Number formatOptions={MAX_FRACTION_DIGITS_FORMAT_OPTIONS} value={row.original.size} />
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
                formatOptions={USD_MAX_FRACTION_DIGITS_FORMAT_OPTIONS}
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
