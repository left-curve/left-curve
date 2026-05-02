import { useState, useMemo } from "react";
import { View, Text } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Decimal } from "@left-curve/dango/utils";
import {
  useConfig,
  useAccount,
  perpsUserStateStore,
  perpsUserStateExtendedStore,
  perpsOrdersByUserStore,
  allPerpsPairStatsStore,
} from "@left-curve/store";
import { Button, Card, Chip, FormattedNumber, Skeleton, Table, Tabs } from "../components";

type Dec = ReturnType<typeof Decimal>;

// ---------------------------------------------------------------------------
// Column widths — shared between header and rows for alignment
// ---------------------------------------------------------------------------

const POSITION_COLS = [
  "w-[160px]", // MARKET
  "w-[120px]", // SIZE / SIDE
  "w-[110px]", // ENTRY
  "w-[110px]", // MARK
  "w-[110px]", // LIQ.
  "w-[100px]", // MARGIN
  "flex-1", // UNREALIZED PNL
  "w-[60px]", // TP/SL
  "w-[70px]", // Close
] as const;

const ORDER_COLS = [
  "w-[160px]", // MARKET
  "w-[100px]", // SIDE
  "w-[100px]", // TYPE
  "w-[120px]", // PRICE
  "w-[100px]", // SIZE
  "flex-1", // actions
] as const;

// ---------------------------------------------------------------------------
// Tab badge — small count pill next to a tab label
// ---------------------------------------------------------------------------

function TabLabel({ label, count }: { readonly label: string; readonly count?: number }) {
  return (
    <View className="flex flex-row items-center gap-1.5">
      <Text className="text-[13px] font-medium text-fg-secondary">{label}</Text>
      {count != null && count > 0 && (
        <View className="bg-bg-tint rounded px-1 min-w-[18px] h-[16px] items-center justify-center">
          <Text className="text-fg-tertiary text-[10px] font-medium tabular-nums">{count}</Text>
        </View>
      )}
    </View>
  );
}

// ---------------------------------------------------------------------------
// Market avatar — colored circle with first letter
// ---------------------------------------------------------------------------

function MarketAvatar({ symbol }: { readonly symbol: string }) {
  return (
    <View className="w-[22px] h-[22px] rounded-full bg-accent-bg items-center justify-center">
      <Text className="text-accent text-[9px] font-semibold">{symbol[0]}</Text>
    </View>
  );
}

// ---------------------------------------------------------------------------
// Position row
// ---------------------------------------------------------------------------

type PositionRow = {
  readonly pairId: string;
  readonly sym: string;
  readonly side: "Long" | "Short";
  readonly size: Dec;
  readonly leverage: number;
  readonly entry: Dec;
  readonly mark: Dec;
  readonly pnl: Dec;
  readonly pnlPct: Dec;
  readonly margin: Dec;
  readonly liqPrice: Dec | null;
};

function PositionRowItem({ position }: { readonly position: PositionRow }) {
  return (
    <Table.Row columns={POSITION_COLS}>
      {/* MARKET */}
      <Table.Cell index={0}>
        <View className="flex flex-row items-center gap-2.5">
          <MarketAvatar symbol={position.sym} />
          <Text className="text-fg-primary text-[12px] font-medium">{position.sym}</Text>
          <Chip variant="outline" className="h-[18px] px-1.5 text-[10px]">
            <Text className="text-fg-secondary text-[10px] font-medium tabular-nums">
              {position.leverage} {"\u00D7"}
            </Text>
          </Chip>
        </View>
      </Table.Cell>

      {/* SIZE / SIDE */}
      <Table.Cell index={1}>
        <Text
          className={twMerge(
            "text-[12px] font-medium tabular-nums",
            position.side === "Long" ? "text-up" : "text-down",
          )}
        >
          {position.side} {"\u00B7"} {position.size.abs().toString()}
        </Text>
      </Table.Cell>

      {/* ENTRY */}
      <Table.Cell index={2}>
        <FormattedNumber
          value={position.entry.toString()}
          formatOptions={{ currency: "USD" }}
          className="text-fg-secondary text-[12px]"
        />
      </Table.Cell>

      {/* MARK */}
      <Table.Cell index={3}>
        <FormattedNumber
          value={position.mark.toString()}
          formatOptions={{ currency: "USD" }}
          className="text-fg-primary text-[12px] font-medium"
        />
      </Table.Cell>

      {/* LIQ. */}
      <Table.Cell index={4}>
        {position.liqPrice ? (
          <FormattedNumber
            value={position.liqPrice.toString()}
            formatOptions={{ currency: "USD" }}
            className="text-down text-[12px]"
          />
        ) : (
          <Text className="text-fg-tertiary text-[12px] tabular-nums font-mono">N/A</Text>
        )}
      </Table.Cell>

      {/* MARGIN */}
      <Table.Cell index={5}>
        <FormattedNumber
          value={position.margin.toString()}
          formatOptions={{ currency: "USD" }}
          className="text-fg-secondary text-[12px]"
        />
      </Table.Cell>

      {/* UNREALIZED PNL */}
      <Table.Cell index={6}>
        <View className="flex flex-row items-baseline gap-1">
          <FormattedNumber
            value={position.pnl.toString()}
            formatOptions={{ currency: "USD" }}
            sign
            colorSign
            className="text-[12px] font-medium"
          />
          <View className="flex flex-row items-baseline">
            <Text className="text-fg-tertiary text-[12px]">(</Text>
            <FormattedNumber
              value={position.pnlPct.toString()}
              formatOptions={{ fractionDigits: 2 }}
              sign
              colorSign
              className="text-fg-tertiary text-[12px]"
            />
            <Text className="text-fg-tertiary text-[12px]">%)</Text>
          </View>
        </View>
      </Table.Cell>

      {/* TP/SL */}
      <Table.Cell index={7}>
        <Text className="text-fg-tertiary text-[12px]">TP/SL</Text>
      </Table.Cell>

      {/* Close */}
      <Table.Cell index={8}>
        <Button variant="secondary" size="sm">
          <Text className="text-fg-primary text-[12px]">Close</Text>
        </Button>
      </Table.Cell>
    </Table.Row>
  );
}

// ---------------------------------------------------------------------------
// Order row
// ---------------------------------------------------------------------------

type OrderRow = {
  readonly id: string;
  readonly sym: string;
  readonly side: "Buy" | "Sell";
  readonly type: string;
  readonly price: Dec;
  readonly size: Dec;
};

function OrderRowItem({ order }: { readonly order: OrderRow }) {
  return (
    <Table.Row columns={ORDER_COLS}>
      <Table.Cell index={0}>
        <View className="flex flex-row items-center gap-2.5">
          <MarketAvatar symbol={order.sym} />
          <Text className="text-fg-primary text-[12px] font-medium">{order.sym}</Text>
        </View>
      </Table.Cell>

      <Table.Cell index={1}>
        <Text
          className={twMerge(
            "text-[12px] font-medium",
            order.side === "Buy" ? "text-up" : "text-down",
          )}
        >
          {order.side}
        </Text>
      </Table.Cell>

      <Table.Cell index={2}>
        <Text className="text-fg-secondary text-[12px]">{order.type}</Text>
      </Table.Cell>

      <Table.Cell index={3}>
        <FormattedNumber
          value={order.price.toString()}
          formatOptions={{ currency: "USD" }}
          className="text-fg-primary text-[12px]"
        />
      </Table.Cell>

      <Table.Cell index={4}>
        <FormattedNumber
          value={order.size.abs().toString()}
          formatOptions={{ fractionDigits: 4 }}
          className="text-fg-primary text-[12px]"
        />
      </Table.Cell>

      <Table.Cell index={5}>
        <View className="flex flex-row justify-end">
          <Button variant="ghost" size="sm">
            <Text className="text-fg-tertiary text-[12px]">{"\u2715"}</Text>
          </Button>
        </View>
      </Table.Cell>
    </Table.Row>
  );
}

// ---------------------------------------------------------------------------
// Skeleton loading state
// ---------------------------------------------------------------------------

function SkeletonTable() {
  return (
    <View className="flex flex-col gap-1 p-4">
      {Array.from({ length: 3 }, (_, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: static skeleton rows
        <Skeleton key={`skeleton-${i}`} height={36} className="w-full" />
      ))}
    </View>
  );
}

// ---------------------------------------------------------------------------
// Positions table
// ---------------------------------------------------------------------------

function PositionsTable({
  loading,
  positions,
}: {
  readonly loading: boolean;
  readonly positions: readonly PositionRow[];
}) {
  return (
    <>
      <Table.Header columns={POSITION_COLS}>
        <Table.HeaderCell index={0}>Market</Table.HeaderCell>
        <Table.HeaderCell index={1}>Size / Side</Table.HeaderCell>
        <Table.HeaderCell index={2}>Entry</Table.HeaderCell>
        <Table.HeaderCell index={3}>Mark</Table.HeaderCell>
        <Table.HeaderCell index={4}>Liq.</Table.HeaderCell>
        <Table.HeaderCell index={5}>Margin</Table.HeaderCell>
        <Table.HeaderCell index={6}>Unrealized PnL</Table.HeaderCell>
        <Table.HeaderCell index={7}>TP/SL</Table.HeaderCell>
        <Table.HeaderCell index={8} />
      </Table.Header>
      {loading ? (
        <SkeletonTable />
      ) : positions.length > 0 ? (
        positions.map((pos) => <PositionRowItem key={pos.pairId} position={pos} />)
      ) : (
        <Table.Empty>No open positions.</Table.Empty>
      )}
    </>
  );
}

// ---------------------------------------------------------------------------
// Orders table
// ---------------------------------------------------------------------------

function OrdersTable({
  loading,
  orders,
}: {
  readonly loading: boolean;
  readonly orders: readonly OrderRow[];
}) {
  return (
    <>
      <Table.Header columns={ORDER_COLS}>
        <Table.HeaderCell index={0}>Market</Table.HeaderCell>
        <Table.HeaderCell index={1}>Side</Table.HeaderCell>
        <Table.HeaderCell index={2}>Type</Table.HeaderCell>
        <Table.HeaderCell index={3}>Price</Table.HeaderCell>
        <Table.HeaderCell index={4}>Size</Table.HeaderCell>
        <Table.HeaderCell index={5} />
      </Table.Header>
      {loading ? (
        <SkeletonTable />
      ) : orders.length > 0 ? (
        orders.map((order) => <OrderRowItem key={order.id} order={order} />)
      ) : (
        <Table.Empty>No open orders.</Table.Empty>
      )}
    </>
  );
}

// ---------------------------------------------------------------------------
// TradeHistory — main export
// ---------------------------------------------------------------------------

export function TradeHistory() {
  const [activeTab, setActiveTab] = useState("positions");
  const { coins } = useConfig();
  const { isConnected } = useAccount();

  const userState = perpsUserStateStore((s) => s.userState);
  const extendedPositions = perpsUserStateExtendedStore((s) => s.positions);
  const perpsOrders = perpsOrdersByUserStore((s) => s.orders);
  const statsByPairId = allPerpsPairStatsStore((s) => s.perpsPairStatsByPairId);

  const symbolToDenom = useMemo(() => {
    const map: Record<string, string> = {};
    for (const [denom, coin] of Object.entries(coins.byDenom)) {
      map[coin.symbol.toLowerCase()] = denom;
    }
    return map;
  }, [coins]);

  const positions: readonly PositionRow[] = useMemo(() => {
    if (!userState?.positions) return [];
    return Object.entries(userState.positions).map(([pairId, pos]) => {
      const baseSymbol = pairId.replace("perp/", "").replace(/usd$/i, "");
      const baseDenom = symbolToDenom[baseSymbol] ?? baseSymbol;
      const coinSymbol = coins.byDenom[baseDenom]?.symbol ?? baseSymbol.toUpperCase();
      const markPrice = Number(statsByPairId[pairId]?.currentPrice ?? pos.entryPrice);
      const sizeNum = Number(pos.size);
      const pnl = sizeNum * (markPrice - Number(pos.entryPrice));
      const entry = Decimal(pos.entryPrice);
      const pnlPct = entry.gt(0)
        ? Decimal(pnl.toString())
            .div(entry.mul(Decimal(Math.abs(sizeNum).toString())))
            .mul(Decimal("100"))
        : Decimal("0");

      const backendLiqPrice = extendedPositions[pairId]?.liquidationPrice;

      return {
        pairId,
        sym: `${coinSymbol}-PERP`,
        side: sizeNum > 0 ? ("Long" as const) : ("Short" as const),
        size: Decimal(pos.size),
        leverage: 10,
        entry,
        mark: Decimal(markPrice.toString()),
        pnl: Decimal(pnl.toString()),
        pnlPct,
        margin: Decimal(pos.size).abs().mul(Decimal(pos.entryPrice)),
        liqPrice: backendLiqPrice != null ? Decimal(backendLiqPrice) : null,
      };
    });
  }, [userState, extendedPositions, statsByPairId, symbolToDenom, coins.byDenom]);

  const openOrders: readonly OrderRow[] = useMemo(() => {
    if (!perpsOrders) return [];
    return Object.entries(perpsOrders).map(([orderId, order]) => {
      const label = order.pairId.replace("perp/", "").replace(/usd$/i, "-PERP").toUpperCase();
      const isLong = Number(order.size) > 0;
      return {
        id: orderId,
        sym: label,
        side: isLong ? ("Buy" as const) : ("Sell" as const),
        type: "Limit",
        price: Decimal(order.limitPrice),
        size: Decimal(order.size),
      };
    });
  }, [perpsOrders]);

  const positionsCount = positions.length;
  const ordersCount = openOrders.length;

  const historyTabs = useMemo(
    () => [
      {
        value: "positions",
        label: <TabLabel label="Positions" count={positionsCount} />,
      },
      {
        value: "orders",
        label: <TabLabel label="Open orders" count={ordersCount} />,
      },
      { value: "fills", label: "Fills" },
      { value: "funding", label: "Funding" },
      { value: "history", label: "History" },
    ],
    [positionsCount, ordersCount],
  );

  const activeTabContent = (() => {
    if (!isConnected) {
      return <Table.Empty>No data (connect wallet)</Table.Empty>;
    }

    switch (activeTab) {
      case "positions":
        return <PositionsTable loading={userState === null} positions={positions} />;
      case "orders":
        return <OrdersTable loading={perpsOrders === null} orders={openOrders} />;
      default:
        return <Table.Empty>No {activeTab} found.</Table.Empty>;
    }
  })();

  return (
    <Card className="flex flex-col h-full overflow-hidden">
      {/* Header: tabs + actions */}
      <View className="flex flex-row items-center justify-between px-4 border-b border-border-subtle">
        <Tabs
          variant="underline"
          items={historyTabs}
          value={activeTab}
          onChange={setActiveTab}
          className="border-b-0"
        />
        <View className="flex flex-row items-center gap-2">
          <Button variant="ghost" size="sm">
            <Text className="text-fg-secondary text-[12px]">Hide other markets</Text>
          </Button>
          <Button variant="secondary" size="sm">
            <Text className="text-fg-primary text-[12px]">Close all</Text>
          </Button>
        </View>
      </View>

      {/* Table content */}
      <Table className="flex-1 overflow-auto">{activeTabContent}</Table>
    </Card>
  );
}
