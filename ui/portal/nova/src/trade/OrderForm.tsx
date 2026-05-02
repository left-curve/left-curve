import { useState, useCallback, useMemo, useEffect } from "react";
import { View, Text, Pressable, TextInput } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Decimal, resolveRateSchedule } from "@left-curve/dango/utils";
import {
  useAccount,
  useAppConfig,
  useTradeCoins,
  usePerpsSubmission,
  usePerpsMaxSize,
  perpsUserStateStore,
  perpsUserStateExtendedStore,
  allPerpsPairStatsStore,
  TradePairStore,
  tradeInfoStore,
  useConfig,
  usePrices,
} from "@left-curve/store";
import { computeOtherPairsUsedMargin } from "../../components/dex/helpers/math";
import {
  Button,
  Card,
  Chip,
  Dropdown,
  DropdownItem,
  FormattedNumber,
  Tabs,
  Toggle,
} from "../components";

const ORDER_TYPE_TABS = [
  { value: "market", label: "Market" },
  { value: "limit", label: "Limit" },
] as const;

const LEVERAGE_STOPS = [1, 5, 10, 25, 50, 100] as const;
const SIZE_PERCENTAGES = [25, 50, 75, 100] as const;

type SummaryRowProps = {
  readonly label: string;
  readonly children: React.ReactNode;
  readonly className?: string;
};

function SummaryRow({ label, children, className }: SummaryRowProps) {
  return (
    <View className="flex flex-row items-baseline justify-between gap-2">
      <Text className="text-fg-tertiary text-[12px] whitespace-nowrap">{label}</Text>
      {typeof children === "string" ? (
        <Text
          className={twMerge(
            "text-fg-primary text-[12px] font-mono tabular-nums whitespace-nowrap",
            className,
          )}
        >
          {children}
        </Text>
      ) : (
        <View className={twMerge("flex flex-row items-baseline", className)}>{children}</View>
      )}
    </View>
  );
}

const MODE_LABELS: Record<string, string> = {
  price: "Price (USD)",
  pnl: "PnL (USD)",
  roi: "ROI (%)",
};

const MODE_DESCRIPTIONS: Record<string, string> = {
  price: "Execute your TP/SL based on the crypto price",
  pnl: "Set TP/SL prices based on estimated PnL",
  roi: "Set TP/SL prices based on estimated ROI%",
};

type TpSlInputProps = {
  readonly label: string;
  readonly value: string;
  readonly onChange: (v: string) => void;
  readonly mode: "price" | "pnl" | "roi";
  readonly onModeChange: (m: "price" | "pnl" | "roi") => void;
  readonly modeOpen: boolean;
  readonly onModeToggle: (open: boolean) => void;
  readonly placeholder: string;
};

function TpSlInput({
  label,
  value,
  onChange,
  mode,
  onModeChange,
  modeOpen,
  onModeToggle,
  placeholder,
}: TpSlInputProps) {
  return (
    <View className="flex flex-col gap-1.5">
      <View className="flex flex-row items-center gap-1.5">
        <Text className="text-fg-secondary text-[12px] font-medium">{label}</Text>
        <Pressable className="flex flex-row items-center gap-0.5">
          <Text className="text-fg-tertiary text-[11px]">Mark</Text>
          <Text className="text-fg-tertiary text-[8px]">{"\u25BE"}</Text>
        </Pressable>
      </View>
      <View className="flex flex-row items-center bg-bg-sunk rounded-field h-9 px-3 border border-border-subtle">
        <TextInput
          value={value}
          onChangeText={onChange}
          placeholder={placeholder}
          placeholderTextColor="var(--fg-quaternary)"
          className="flex-1 bg-transparent border-0 outline-none text-[13px] text-fg-primary font-mono tabular-nums min-w-0"
        />
        <Dropdown
          open={modeOpen}
          onOpenChange={onModeToggle}
          align="right"
          trigger={
            <Pressable
              className="flex flex-row items-center gap-0.5 shrink-0 ml-2"
              onPress={() => onModeToggle(!modeOpen)}
            >
              <Text className="text-fg-tertiary text-[11px] whitespace-nowrap">
                {MODE_LABELS[mode]}
              </Text>
              <Text className="text-fg-tertiary text-[8px]">{"\u25BE"}</Text>
            </Pressable>
          }
        >
          {(["price", "pnl", "roi"] as const).map((m) => (
            <DropdownItem
              key={m}
              selected={mode === m}
              onPress={() => {
                onModeChange(m);
                onModeToggle(false);
              }}
            >
              <Text className="text-fg-primary text-[12px] font-medium">{MODE_LABELS[m]}</Text>
              <Text className="text-fg-tertiary text-[11px]">{MODE_DESCRIPTIONS[m]}</Text>
            </DropdownItem>
          ))}
        </Dropdown>
      </View>
    </View>
  );
}

export function OrderForm() {
  const { isConnected } = useAccount();
  const { data: appConfig } = useAppConfig();
  const { baseCoin } = useTradeCoins();
  const { getPrice } = usePrices();
  const { coins: allCoins } = useConfig();

  const getPerpsPairId = TradePairStore((s) => s.getPerpsPairId);
  const perpsPairId = getPerpsPairId();

  const side = tradeInfoStore((s) => s.action);
  const setSide = tradeInfoStore((s) => s.setAction);
  const orderType = tradeInfoStore((s) => s.operation);
  const setOrderType = tradeInfoStore((s) => s.setOperation);

  const [price, setPrice] = useState("");
  const [size, setSize] = useState("");
  const [sizeMode, setSizeMode] = useState<"base" | "quote">("base");
  const [postOnly, setPostOnly] = useState(false);
  const [reduceOnly, setReduceOnly] = useState(false);
  const [tpslEnabled, setTpslEnabled] = useState(false);
  const [tpValue, setTpValue] = useState("");
  const [slValue, setSlValue] = useState("");
  const [tpMode, setTpMode] = useState<"price" | "pnl" | "roi">("pnl");
  const [slMode, setSlMode] = useState<"price" | "pnl" | "roi">("pnl");
  const [tpModeOpen, setTpModeOpen] = useState(false);
  const [slModeOpen, setSlModeOpen] = useState(false);

  const statsByPairId = allPerpsPairStatsStore((s) => s.perpsPairStatsByPairId);
  const userState = perpsUserStateStore((s) => s.userState);
  const equity = perpsUserStateExtendedStore((s) => s.equity) ?? "0";

  const params = appConfig.perpsPairs?.[perpsPairId];

  const currentPrice = useMemo(() => {
    const fromStats = Number(statsByPairId[perpsPairId]?.currentPrice ?? 0);
    if (fromStats > 0) return fromStats;
    const oraclePrice = Number(getPrice(1, baseCoin.denom) ?? 0);
    return Number.isFinite(oraclePrice) ? oraclePrice : 0;
  }, [statsByPairId, perpsPairId, getPrice, baseCoin.denom]);

  useEffect(() => {
    if (currentPrice > 0 && !price) {
      setPrice(currentPrice.toFixed(2));
    }
  }, [currentPrice]);

  const maxLeverage = useMemo(() => {
    if (!params?.initialMarginRatio) return 100;
    const ratio = Number(params.initialMarginRatio);
    return ratio > 0 ? Math.floor(1 / ratio) : 100;
  }, [params]);

  const [leverage, setLeverage] = useState(10);

  useEffect(() => {
    if (leverage > maxLeverage) setLeverage(maxLeverage);
  }, [maxLeverage]);

  const takerFeeRate = useMemo(() => {
    const schedule = appConfig?.perpsParam?.takerFeeRates;
    if (!schedule) return 0;
    const rate = Number(resolveRateSchedule(schedule, "0"));
    return Number.isFinite(rate) ? rate : 0;
  }, [appConfig?.perpsParam]);

  const reservedMargin = Number(userState?.reservedMargin ?? "0");

  const otherPairsUsedMargin = useMemo(() => {
    const positions = userState?.positions;
    if (!positions) return 0;
    return computeOtherPairsUsedMargin(positions, perpsPairId, appConfig.perpsPairs, (pid) => {
      const statsPrice = Decimal(statsByPairId[pid]?.currentPrice ?? 0);
      if (statsPrice.gt(0)) return statsPrice;
      const symbol = pid.match(/^perp\/(.+)usd$/)?.[1]?.toUpperCase();
      const denom = symbol ? allCoins.bySymbol[symbol]?.denom : undefined;
      return denom ? Decimal(getPrice(1, denom) ?? 0) : Decimal(0);
    });
  }, [
    userState?.positions,
    perpsPairId,
    appConfig.perpsPairs,
    statsByPairId,
    allCoins.bySymbol,
    getPrice,
  ]);

  const currentPositionSize = Number(userState?.positions?.[perpsPairId]?.size ?? "0");

  const { availToTrade, maxSize } = usePerpsMaxSize({
    equity: Number(equity),
    reservedMargin,
    otherPairsUsedMargin,
    currentPositionSize,
    action: side,
    leverage,
    currentPrice,
    takerFeeRate,
    reduceOnly,
    isBaseSize: true,
  });

  const sliderPct = maxLeverage > 1 ? ((leverage - 1) / (maxLeverage - 1)) * 100 : 0;

  const sizeDecimal = size ? Decimal(size) : Decimal("0");
  const markDecimal = Decimal(currentPrice > 0 ? currentPrice.toString() : "0");
  const baseSizeDecimal =
    sizeMode === "quote" && markDecimal.gt(0) ? sizeDecimal.div(markDecimal) : sizeDecimal;
  const orderValue = baseSizeDecimal.mul(markDecimal);
  const marginReq = leverage > 0 ? orderValue.div(Decimal(leverage.toString())) : Decimal("0");
  const takerFee = orderValue.mul(Decimal(takerFeeRate.toString()));

  const handleSizePercent = useCallback(
    (pct: number) => {
      const baseResult = Decimal(maxSize.toString())
        .mul(Decimal(pct.toString()))
        .div(Decimal("100"));
      if (sizeMode === "quote" && markDecimal.gt(0)) {
        setSize(baseResult.mul(markDecimal).toFixed(2));
      } else {
        setSize(baseResult.toFixed(4));
      }
    },
    [maxSize, sizeMode, markDecimal],
  );

  const handleOrderTypeChange = useCallback(
    (val: string) => {
      setOrderType(val as "market" | "limit");
    },
    [setOrderType],
  );

  const showPriceInput = orderType !== "market";

  const controllers = useMemo(
    () => ({
      reset: () => {
        setSize("");
        setPrice("");
      },
    }),
    [],
  );

  const submissionSizeValue = sizeMode === "quote" ? baseSizeDecimal.toFixed(8) : size || "0";

  const submission = usePerpsSubmission({
    perpsPairId,
    action: side,
    operation: orderType,
    sizeValue: submissionSizeValue,
    priceValue: price || "0",
    maxSlippage: "0.01",
    reduceOnly,
    controllers,
  });

  const isSubmitDisabled =
    baseSizeDecimal.lte(0) || (orderType === "limit" && Decimal(price || "0").lte(0));

  return (
    <Card className="flex flex-col p-3 gap-3 h-full overflow-auto">
      <View className="flex flex-row gap-1.5 p-1 bg-bg-sunk border border-border-subtle rounded-btn">
        <Pressable
          onPress={() => setSide("buy")}
          className={twMerge(
            "flex-1 h-8 items-center justify-center rounded-[calc(var(--r-btn)-2px)]",
            "transition-[background,color] duration-150 ease-[var(--ease)]",
            side === "buy" ? "bg-up" : "bg-transparent",
          )}
        >
          <Text
            className={twMerge(
              "font-semibold text-[13px]",
              side === "buy" ? "text-white" : "text-fg-tertiary",
            )}
          >
            Buy / Long
          </Text>
        </Pressable>
        <Pressable
          onPress={() => setSide("sell")}
          className={twMerge(
            "flex-1 h-8 items-center justify-center rounded-[calc(var(--r-btn)-2px)]",
            "transition-[background,color] duration-150 ease-[var(--ease)]",
            side === "sell" ? "bg-down" : "bg-transparent",
          )}
        >
          <Text
            className={twMerge(
              "font-semibold text-[13px]",
              side === "sell" ? "text-white" : "text-fg-tertiary",
            )}
          >
            Sell / Short
          </Text>
        </Pressable>
      </View>

      <Tabs
        variant="underline"
        items={[...ORDER_TYPE_TABS]}
        value={orderType}
        onChange={handleOrderTypeChange}
        className="w-full"
        itemClassName="flex-1"
      />

      <View className="flex flex-col gap-2">
        <View className="flex flex-row items-center justify-between">
          <Text className="text-[11px] text-fg-tertiary tracking-wide uppercase font-medium">
            Leverage
          </Text>
          <View className="flex flex-row items-center gap-2">
            <Chip variant="outline">Cross</Chip>
            <Text className="font-mono tabular-nums font-semibold text-fg-primary text-[12px]">
              {leverage}
              {"\u00D7"}
            </Text>
          </View>
        </View>

        <View className="relative h-5 justify-center">
          <View className="h-1 bg-bg-tint rounded-full">
            <View className="h-full bg-accent rounded-full" style={{ width: `${sliderPct}%` }} />
          </View>
          <View
            className="absolute w-3.5 h-3.5 -ml-[7px] rounded-full bg-bg-elev border-2 border-accent shadow-sm pointer-events-none"
            style={{ left: `${sliderPct}%`, top: 3 }}
          />
          <View className="absolute inset-x-0 top-[7px] flex flex-row justify-between pointer-events-none">
            {[0, 25, 50, 75, 100].map((pct) => (
              <View key={pct} className="w-1 h-1 rounded-full bg-border-strong" />
            ))}
          </View>
          <input
            type="range"
            min={1}
            max={maxLeverage}
            value={leverage}
            onChange={(e) => setLeverage(Number(e.target.value))}
            className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
            style={{ margin: 0 }}
          />
        </View>

        <View className="flex flex-row justify-between">
          {LEVERAGE_STOPS.filter((s) => s <= maxLeverage).map((stop) => (
            <Pressable key={stop} onPress={() => setLeverage(stop)}>
              <Text
                className={twMerge(
                  "text-[10px] font-medium tabular-nums font-mono",
                  leverage === stop ? "text-fg-primary" : "text-fg-tertiary",
                )}
              >
                {stop}
                {"\u00D7"}
              </Text>
            </Pressable>
          ))}
        </View>
      </View>

      {showPriceInput && (
        <View className="flex flex-col gap-1 bg-bg-sunk rounded-field p-3">
          <Text className="text-[11px] text-fg-tertiary tracking-wide uppercase font-medium">
            Price (USD)
          </Text>
          <View className="flex flex-row items-center">
            <TextInput
              value={price}
              onChangeText={setPrice}
              className="flex-1 bg-transparent border-0 outline-none text-[16px] font-semibold text-fg-primary font-mono tabular-nums"
            />
            <Text className="text-fg-tertiary text-[12px] font-normal">USD</Text>
          </View>
        </View>
      )}

      <View className="flex flex-col gap-1 bg-bg-sunk rounded-field p-3">
        <View className="flex flex-row items-center justify-between">
          <Text className="text-[11px] text-fg-tertiary tracking-wide uppercase font-medium">
            Size ({sizeMode === "quote" ? "USD" : baseCoin.symbol})
          </Text>
          <Tabs
            items={[
              { value: "base", label: baseCoin.symbol },
              { value: "quote", label: "USD" },
            ]}
            value={sizeMode}
            onChange={(v) => {
              setSizeMode(v as "base" | "quote");
              setSize("");
            }}
            className="flex-row"
          />
        </View>
        <View className="flex flex-row items-center">
          <TextInput
            value={size}
            onChangeText={setSize}
            placeholder="0.00"
            placeholderTextColor="var(--fg-quaternary)"
            className="flex-1 bg-transparent border-0 outline-none text-[16px] font-semibold text-fg-primary font-mono tabular-nums"
          />
          <Text className="text-fg-tertiary text-[12px] font-normal">
            {sizeMode === "quote" ? "USD" : baseCoin.symbol}
          </Text>
        </View>
      </View>

      <View
        className="flex flex-row gap-1.5"
        style={{ display: "grid" as never, gridTemplateColumns: "repeat(4, 1fr)" }}
      >
        {SIZE_PERCENTAGES.map((pct) => (
          <Button key={pct} variant="secondary" size="sm" onPress={() => handleSizePercent(pct)}>
            <Text className="text-fg-primary text-[12px]">{pct}%</Text>
          </Button>
        ))}
      </View>

      <View className="flex flex-col gap-2 pt-1">
        <View className="flex flex-row items-center justify-between">
          <Text className="text-fg-secondary text-[12px]">Post-only</Text>
          <Toggle checked={postOnly} onChange={setPostOnly} />
        </View>
        <View className="flex flex-row items-center justify-between">
          <Text className="text-fg-secondary text-[12px]">Reduce-only</Text>
          <Toggle checked={reduceOnly} onChange={setReduceOnly} />
        </View>
      </View>

      {/* TP/SL Section */}
      <View className="flex flex-col gap-2">
        <View className="flex flex-row items-center justify-between">
          <Text className="text-fg-secondary text-[12px]">TP/SL</Text>
          <Toggle checked={tpslEnabled} onChange={setTpslEnabled} />
        </View>

        {tpslEnabled && (
          <View className="flex flex-col gap-3">
            <TpSlInput
              label="Take Profit"
              value={tpValue}
              onChange={setTpValue}
              mode={tpMode}
              onModeChange={setTpMode}
              modeOpen={tpModeOpen}
              onModeToggle={(open) => {
                setTpModeOpen(open);
                if (open) setSlModeOpen(false);
              }}
              placeholder="TP"
            />
            <TpSlInput
              label="Stop Loss"
              value={slValue}
              onChange={setSlValue}
              mode={slMode}
              onModeChange={setSlMode}
              modeOpen={slModeOpen}
              onModeToggle={(open) => {
                setSlModeOpen(open);
                if (open) setTpModeOpen(false);
              }}
              placeholder="SL"
            />
          </View>
        )}
      </View>

      <View className="flex flex-col gap-1.5 p-2.5 bg-bg-sunk rounded-field">
        <SummaryRow label="Order value">
          <FormattedNumber
            value={orderValue.toString()}
            formatOptions={{ currency: "USD" }}
            className="text-fg-primary text-[12px]"
          />
        </SummaryRow>
        <SummaryRow label="Margin req.">
          <FormattedNumber
            value={marginReq.toString()}
            formatOptions={{ currency: "USD" }}
            className="text-fg-primary text-[12px]"
          />
        </SummaryRow>
        <SummaryRow label="Liq. est.">--</SummaryRow>
        <SummaryRow label="Taker fee" className="text-fg-secondary">
          <FormattedNumber
            value={takerFee.toString()}
            formatOptions={{ currency: "USD" }}
            className="text-fg-secondary text-[12px]"
          />
        </SummaryRow>
      </View>

      <View className="mt-auto">
        <Button
          variant={side === "buy" ? "up" : "down"}
          size="lg"
          className="w-full"
          disabled={!isConnected || isSubmitDisabled}
          onPress={() => submission.mutateAsync()}
        >
          <Text className="text-white font-semibold text-[14px]">
            {side === "buy" ? "Open long" : "Open short"} {"\u00B7"} {baseCoin.symbol}-PERP
          </Text>
        </Button>
      </View>

      <View className="flex flex-col gap-1 p-2.5 border border-border-subtle rounded-field">
        <View className="flex flex-row items-center justify-between">
          <Text className="text-fg-tertiary text-[12px]">Available margin</Text>
          <FormattedNumber
            value={Math.max(availToTrade, 0).toString()}
            formatOptions={{ currency: "USD" }}
            className="text-fg-primary text-[12px]"
          />
        </View>
        <View className="flex flex-row items-center justify-between">
          <Text className="text-fg-tertiary text-[12px]">Account leverage</Text>
          <Text className="text-fg-primary text-[12px] font-mono tabular-nums">{leverage}x</Text>
        </View>
      </View>
    </Card>
  );
}
