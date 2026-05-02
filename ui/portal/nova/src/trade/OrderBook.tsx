import { useEffect, useMemo, useState } from "react";
import { View, Text, Pressable } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Decimal } from "@left-curve/dango/utils";
import {
  useCurrentPrice,
  useTradeCoins,
  useAppConfig,
  usePerpsLiquidityDepth,
  perpsLiquidityDepthStore,
  TradePairStore,
  useStorage,
} from "@left-curve/store";
import { Card, Dropdown, DropdownItem, FormattedNumber, Skeleton, Tabs } from "../components";

type Dec = ReturnType<typeof Decimal>;

type DisplayMode = "base" | "quote";

type BookRowProps = {
  readonly price: Dec;
  readonly size: Dec;
  readonly total: Dec;
  readonly highestSize: Dec;
  readonly side: "ask" | "bid";
  readonly priceFractionDigits: number;
};

function BookRow({ price, size, total, highestSize, side, priceFractionDigits }: BookRowProps) {
  const fillPct = highestSize.gt(Decimal("0"))
    ? Number(size.div(highestSize).mul(Decimal("100")).toFixed(1))
    : 0;

  return (
    <Pressable className="relative flex flex-1 flex-row items-center px-2.5 min-h-[18px] hover:bg-bg-sunk transition-[background] duration-100 ease-[var(--ease)]">
      <View
        className={twMerge("absolute inset-0", side === "ask" ? "bg-down-bg" : "bg-up-bg")}
        style={{ width: `${fillPct}%`, right: 0, left: "auto" }}
      />
      <FormattedNumber
        value={price.toString()}
        formatOptions={{ fractionDigits: priceFractionDigits }}
        className={twMerge("flex-1 text-[12px] relative", side === "ask" ? "text-down" : "text-up")}
      />
      <FormattedNumber
        value={size.toString()}
        formatOptions={{ fractionDigits: 3 }}
        className="flex-1 justify-end text-[12px] text-fg-primary relative"
      />
      <FormattedNumber
        value={total.toString()}
        formatOptions={{ fractionDigits: 3 }}
        className="flex-1 justify-end text-[12px] text-fg-tertiary relative"
      />
    </Pressable>
  );
}

function SkeletonRows({ count }: { readonly count: number }) {
  return (
    <>
      {Array.from({ length: count }, (_, i) => (
        <View
          // biome-ignore lint/suspicious/noArrayIndexKey: static skeleton rows
          key={`skeleton-${i}`}
          className="flex flex-1 flex-row items-center px-2.5 min-h-[18px] gap-2"
        >
          <Skeleton className="flex-1" height={12} />
          <Skeleton className="flex-1" height={12} />
          <Skeleton className="flex-1" height={12} />
        </View>
      ))}
    </>
  );
}

type OrderBookEntry = {
  readonly price: string;
  readonly size: string;
  readonly total: string;
};

type BuildSideResult = {
  readonly records: readonly OrderBookEntry[];
  readonly highestSize: string;
};

function buildSide(
  entries: Record<string, { size: string; notional: string }>,
  direction: "bid" | "ask",
  limit: number,
  displayMode: DisplayMode,
): BuildSideResult {
  const sorted = Object.entries(entries)
    .sort(([a], [b]) =>
      direction === "bid" ? (Decimal(a).gt(b) ? -1 : 1) : Decimal(a).gt(b) ? 1 : -1,
    )
    .slice(0, limit);

  let total = Decimal("0");
  let highestSize = Decimal("0");
  const records = sorted.map(([price, { size, notional }]) => {
    const value = displayMode === "quote" ? notional : size;
    const decValue = Decimal(value);
    total = total.plus(decValue);
    if (decValue.gt(highestSize)) highestSize = decValue;
    return { price, size: value, total: total.toString() };
  });
  return { records, highestSize: highestSize.toString() };
}

type ViewMode = "both" | "bids" | "asks";

const VIEW_TABS = [
  { value: "both", label: "\u2261" },
  { value: "bids", label: "\u2191" },
  { value: "asks", label: "\u2193" },
] as const;

const ROWS_PER_SIDE = 10;

/**
 * Derive the number of fractional digits to show for prices based on
 * the bucket size string (e.g. "0.01" -> 2, "1" -> 0, "0.1" -> 1).
 */
function bucketSizeToFractionDigits(bucketSize: string): number {
  const dotIdx = bucketSize.indexOf(".");
  if (dotIdx === -1) return 0;
  return bucketSize.length - dotIdx - 1;
}

export function OrderBook() {
  const { baseCoin } = useTradeCoins();
  const { currentPrice, previousPrice } = useCurrentPrice();
  const mode = TradePairStore((s) => s.mode);
  const getPerpsPairId = TradePairStore((s) => s.getPerpsPairId);

  const { data: appConfig } = useAppConfig();

  const [viewMode, setViewMode] = useState<ViewMode>("both");

  // --- Perps subscription setup ---
  const perpsPairId = getPerpsPairId();
  const perpsPairConfig = appConfig.perpsPairs?.[perpsPairId];
  const bucketSizes: readonly string[] = perpsPairConfig?.bucketSizes ?? [];

  const [bucketSize, setBucketSize] = useState(bucketSizes[0] ?? "1");

  useEffect(() => {
    setBucketSize(bucketSizes[0] ?? "1");
  }, [bucketSizes]);

  const [displayModeRaw, setDisplayMode] = useStorage<DisplayMode>("nova-order-book-display-mode", {
    initialValue: "base",
  });
  const displayMode: DisplayMode = displayModeRaw === "quote" ? "quote" : "base";

  // Activate the WebSocket subscription for perps liquidity depth
  usePerpsLiquidityDepth({
    pairId: perpsPairId,
    bucketSize,
    subscribe: mode === "perps" && perpsPairId !== "",
  });

  const depthData = perpsLiquidityDepthStore((s) => s.liquidityDepth);
  const hasData = mode === "perps" && depthData !== null;

  const priceFractionDigits = useMemo(() => bucketSizeToFractionDigits(bucketSize), [bucketSize]);

  const rowLimit = viewMode === "both" ? ROWS_PER_SIDE : ROWS_PER_SIDE * 2;

  const { askSide, bidSide } = useMemo(() => {
    if (!hasData) {
      const empty: BuildSideResult = { records: [], highestSize: "0" };
      return { askSide: empty, bidSide: empty };
    }
    return {
      askSide:
        viewMode === "bids"
          ? { records: [], highestSize: "0" }
          : buildSide(depthData.asks, "ask", rowLimit, displayMode),
      bidSide:
        viewMode === "asks"
          ? { records: [], highestSize: "0" }
          : buildSide(depthData.bids, "bid", rowLimit, displayMode),
    };
  }, [hasData, depthData, viewMode, rowLimit, displayMode]);

  const asks = askSide.records;
  const bids = bidSide.records;

  const highestSize = useMemo(() => {
    const askMax = Decimal(askSide.highestSize);
    const bidMax = Decimal(bidSide.highestSize);
    return askMax.gt(bidMax) ? askMax : bidMax;
  }, [askSide.highestSize, bidSide.highestSize]);

  const spread = useMemo(() => {
    if (!hasData) return null;
    const bidPrices = Object.keys(depthData.bids);
    const askPrices = Object.keys(depthData.asks);
    if (!bidPrices.length || !askPrices.length) return null;
    const bestBid = bidPrices.reduce((max, p) => (Decimal(p).gt(max) ? p : max), bidPrices[0]);
    const bestAsk = askPrices.reduce((min, p) => (Decimal(p).lt(min) ? p : min), askPrices[0]);
    return Decimal(bestAsk).minus(Decimal(bestBid));
  }, [hasData, depthData]);

  const midPrice = currentPrice ? Decimal(currentPrice) : null;
  const isUp = previousPrice && currentPrice ? Decimal(previousPrice).lte(currentPrice) : true;

  const showAsks = viewMode !== "bids";
  const showBids = viewMode !== "asks";

  const sizeSymbol = displayMode === "quote" ? "USD" : baseCoin.symbol;

  const [bucketOpen, setBucketOpen] = useState(false);

  return (
    <Card className="flex flex-col h-full overflow-hidden">
      {/* Row 1: Order Book / Trades tabs */}
      <Tabs
        items={[
          { value: "book", label: "Order Book" },
          { value: "trades", label: "Trades" },
        ]}
        value="book"
        className="flex-row w-full"
        itemClassName="flex-1"
      />

      {/* Row 2: View mode arrows + bucket size + token toggle */}
      <View className="flex flex-row items-center justify-between px-2.5 py-1.5 border-b border-border-subtle">
        <Tabs
          items={[...VIEW_TABS]}
          value={viewMode}
          onChange={(v) => setViewMode(v as ViewMode)}
          className="flex-row"
        />
        <View className="flex flex-row items-center gap-1.5">
          {bucketSizes.length > 0 && (
            <Dropdown
              open={bucketOpen}
              onOpenChange={setBucketOpen}
              align="right"
              trigger={
                <Pressable
                  onPress={() => setBucketOpen((o) => !o)}
                  className="flex flex-row items-center gap-1 h-[22px] px-2 rounded-chip border border-border-subtle bg-bg-tint"
                >
                  <Text className="text-[11px] font-medium text-fg-secondary font-mono tabular-nums">
                    {bucketSize}
                  </Text>
                  <Text className="text-fg-tertiary text-[8px]">{"\u25BE"}</Text>
                </Pressable>
              }
            >
              {bucketSizes.map((size) => (
                <DropdownItem
                  key={size}
                  selected={size === bucketSize}
                  onPress={() => {
                    setBucketSize(size);
                    setBucketOpen(false);
                  }}
                  className="px-3 py-1"
                >
                  <Text className="text-[11px] font-mono tabular-nums text-fg-primary">{size}</Text>
                </DropdownItem>
              ))}
            </Dropdown>
          )}
          <Tabs
            items={[
              { value: "base", label: baseCoin.symbol },
              { value: "quote", label: "USD" },
            ]}
            value={displayMode}
            onChange={(v) => setDisplayMode(v as DisplayMode)}
            className="flex-row"
          />
        </View>
      </View>

      {/* Column headers */}
      <View className="flex flex-row items-center px-2.5 py-1">
        <Text className="flex-1 text-[10px] text-fg-tertiary tracking-wide uppercase">
          Price (USD)
        </Text>
        <Text className="flex-1 text-right text-[10px] text-fg-tertiary tracking-wide uppercase">
          Size ({sizeSymbol})
        </Text>
        <Text className="flex-1 text-right text-[10px] text-fg-tertiary tracking-wide uppercase">
          Total ({sizeSymbol})
        </Text>
      </View>

      {/* Ask side */}
      {showAsks && (
        <View className="flex-1 overflow-hidden pb-1 justify-end">
          {hasData && asks.length > 0 ? (
            [...asks]
              .reverse()
              .map((row) => (
                <BookRow
                  key={`ask-${row.price}`}
                  price={Decimal(row.price)}
                  size={Decimal(row.size)}
                  total={Decimal(row.total)}
                  highestSize={highestSize}
                  side="ask"
                  priceFractionDigits={priceFractionDigits}
                />
              ))
          ) : (
            <SkeletonRows count={viewMode === "asks" ? ROWS_PER_SIDE * 2 : ROWS_PER_SIDE} />
          )}
        </View>
      )}

      {/* Spread row */}
      <View className="flex flex-row items-center px-2.5 py-1.5 border-t border-b border-border-subtle bg-bg-sunk">
        {midPrice ? (
          <>
            <FormattedNumber
              value={midPrice.toString()}
              formatOptions={{ fractionDigits: priceFractionDigits }}
              className={twMerge("font-medium text-[13px]", isUp ? "text-up" : "text-down")}
            />
            <Text className="text-fg-tertiary text-[11px] ml-1">{isUp ? "\u2191" : "\u2193"}</Text>
            <View className="flex flex-row items-baseline ml-1.5">
              <Text className="text-fg-tertiary text-[12px] font-mono tabular-nums">
                {"\u2248"}{" "}
              </Text>
              <FormattedNumber
                value={midPrice.toString()}
                formatOptions={{ currency: "USD" }}
                className="text-fg-tertiary text-[12px]"
              />
            </View>
          </>
        ) : (
          <Skeleton width={120} height={16} />
        )}
        <View className="flex-1" />
        {spread && (
          <View className="flex flex-row items-baseline gap-1">
            <Text className="text-fg-tertiary text-[11px]">Spread</Text>
            <FormattedNumber
              value={spread.toString()}
              formatOptions={{ fractionDigits: priceFractionDigits }}
              className="text-fg-tertiary text-[11px]"
            />
          </View>
        )}
      </View>

      {/* Bid side */}
      {showBids && (
        <View className="flex-1 overflow-hidden pt-1">
          {hasData && bids.length > 0 ? (
            bids.map((row) => (
              <BookRow
                key={`bid-${row.price}`}
                price={Decimal(row.price)}
                size={Decimal(row.size)}
                total={Decimal(row.total)}
                highestSize={highestSize}
                side="bid"
                priceFractionDigits={priceFractionDigits}
              />
            ))
          ) : (
            <SkeletonRows count={viewMode === "bids" ? ROWS_PER_SIDE * 2 : ROWS_PER_SIDE} />
          )}
        </View>
      )}
    </Card>
  );
}
