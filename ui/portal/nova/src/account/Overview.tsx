import { View, Text } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Decimal, formatUnits } from "@left-curve/dango/utils";
import { useAccount, useBalances, useConfig, usePrices } from "@left-curve/store";
import { useMemo } from "react";
import { Button, Card, Skeleton, Tabs } from "../components";
import { SearchPalette } from "../layout/SearchPalette";

const HOLDINGS_TABS = [
  { value: "all", label: "All" },
  { value: "spot", label: "Spot" },
  { value: "earning", label: "Earning" },
] as const;

const ACTIVITY_TABS = [
  { value: "all", label: "All" },
  { value: "trades", label: "Trades" },
  { value: "transfers", label: "Transfers" },
  { value: "funding", label: "Funding" },
] as const;

type DecimalValue = ReturnType<typeof Decimal>;

type AssetRow = {
  sym: string;
  name: string;
  denom: string;
  balance: DecimalValue;
  price: DecimalValue;
  value: DecimalValue;
  logoURI?: string;
};

function formatUsd(value: DecimalValue, decimals = 2): string {
  const num = Number(value.toFixed(decimals));
  return `$${num.toLocaleString("en-US", { minimumFractionDigits: decimals, maximumFractionDigits: decimals })}`;
}

function formatBalance(value: DecimalValue): string {
  return Number(value.lt(Decimal("10")) ? value.toFixed(4) : value.toFixed(2)).toLocaleString(
    "en-US",
  );
}

function SparklinePlaceholder({ color }: { color: string }) {
  return (
    <View
      className={twMerge(
        "h-[36px] w-[120px] rounded-field",
        color === "up" ? "bg-up-bg" : "bg-down-bg",
      )}
      style={{ opacity: 0.4 }}
    />
  );
}

function KpiCard({
  label,
  children,
  className,
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <Card className={twMerge("p-[18px]", className)}>
      <Text className="text-[11px] text-fg-tertiary tracking-wide uppercase font-medium">
        {label}
      </Text>
      {children}
    </Card>
  );
}

function HeroSearch() {
  return <SearchPalette.Hero />;
}

function HoldingsRow({ asset }: { asset: AssetRow }) {
  return (
    <View className="flex flex-row items-center h-12 px-4 hover:bg-bg-sunk transition-[background] duration-150 ease-[var(--ease)]">
      <View className="flex flex-row items-center gap-2.5 w-[180px]">
        <View className="w-7 h-7 rounded-full bg-bg-tint items-center justify-center">
          <Text className="text-fg-secondary text-[11px] font-semibold">{asset.sym[0]}</Text>
        </View>
        <View className="flex flex-col">
          <Text className="text-fg-primary text-[13px] font-medium">{asset.sym}</Text>
          <Text className="text-fg-tertiary text-[11px]">{asset.name}</Text>
        </View>
      </View>

      <Text className="w-[120px] text-fg-primary text-[13px] tabular-nums">
        {formatBalance(asset.balance)}
      </Text>

      <Text className="w-[120px] text-fg-secondary text-[13px] tabular-nums">
        {formatUsd(asset.price, asset.price.lt(Decimal("10")) ? 4 : 2)}
      </Text>

      <View className="w-[80px]">
        <Text className="text-fg-tertiary text-[13px] tabular-nums">{"\u2014"}</Text>
      </View>

      <Text className="w-[120px] text-fg-primary text-[13px] font-medium tabular-nums">
        {formatUsd(asset.value)}
      </Text>

      <View className="w-[120px]">
        <SparklinePlaceholder color="up" />
      </View>

      <View className="flex-1 flex flex-row justify-end gap-1">
        <Button variant="ghost" size="sm">
          <Text className="text-fg-secondary text-[12px]">Trade</Text>
        </Button>
        <Button variant="ghost" size="sm">
          <Text className="text-fg-secondary text-[12px]">Send</Text>
        </Button>
      </View>
    </View>
  );
}

function HoldingsTable({ assets, isLoading }: { assets: readonly AssetRow[]; isLoading: boolean }) {
  return (
    <Card className="overflow-hidden">
      <View className="flex flex-row items-center justify-between px-4 py-3">
        <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">Holdings</Text>
        <Tabs items={[...HOLDINGS_TABS]} defaultValue="all" />
      </View>

      <View className="flex flex-row items-center h-8 px-4 border-t border-border-subtle bg-bg-sunk">
        <Text className="w-[180px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
          Asset
        </Text>
        <Text className="w-[120px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
          Balance
        </Text>
        <Text className="w-[120px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
          Price
        </Text>
        <Text className="w-[80px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
          24h
        </Text>
        <Text className="w-[120px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
          Value
        </Text>
        <Text className="w-[120px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
          30d
        </Text>
        <View className="flex-1" />
      </View>

      {isLoading ? (
        <View className="flex flex-col gap-2 p-4">
          <Skeleton height={48} />
          <Skeleton height={48} />
          <Skeleton height={48} />
        </View>
      ) : assets.length === 0 ? (
        <View className="p-8 items-center">
          <Text className="text-fg-tertiary text-[13px]">No assets found</Text>
        </View>
      ) : (
        assets.map((asset) => <HoldingsRow key={asset.denom} asset={asset} />)
      )}
    </Card>
  );
}

function ActivityTable() {
  return (
    <Card className="overflow-hidden">
      <View className="flex flex-row items-center justify-between px-4 py-3">
        <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">
          Recent activity
        </Text>
        <Tabs items={[...ACTIVITY_TABS]} defaultValue="all" />
      </View>

      <View className="p-8 items-center">
        <Text className="text-fg-tertiary text-[13px]">No recent activity</Text>
      </View>
    </Card>
  );
}

export function Overview() {
  const { account } = useAccount();
  const { coins } = useConfig();
  const { getPrice } = usePrices();
  const { data: balances = {}, isLoading } = useBalances({ address: account?.address });

  const assets = useMemo<AssetRow[]>(() => {
    return Object.entries(balances)
      .map(([denom, amount]) => {
        const coinInfo = coins.getCoinInfo(denom);
        const humanAmount = formatUnits(amount, coinInfo.decimals);
        const priceNum = getPrice(humanAmount, denom);
        return {
          sym: coinInfo.symbol,
          name: coinInfo.name,
          denom,
          balance: Decimal(humanAmount),
          price: Decimal(String(getPrice("1", denom))),
          value: Decimal(String(priceNum)),
          logoURI: coinInfo.logoURI,
        };
      })
      .filter((a) => a.value.gt(Decimal("0")) || a.balance.gt(Decimal("0")))
      .sort((a, b) => Number(b.value.minus(a.value).toFixed(2)));
  }, [balances, coins, getPrice]);

  const totalEquity = useMemo(
    () => assets.reduce((sum, a) => sum.plus(a.value), Decimal("0")),
    [assets],
  );

  const spotTotal = totalEquity;

  return (
    <View className="flex flex-col gap-4">
      <HeroSearch />

      <View className="grid grid-cols-1 md:grid-cols-4 gap-3" style={{ display: "grid" as never }}>
        <KpiCard label="Total equity" className="md:col-span-1">
          <View className="flex flex-row items-baseline gap-3 mt-1.5">
            {isLoading ? (
              <Skeleton height={32} width={160} />
            ) : (
              <Text className="text-fg-primary font-semibold text-[28px] tracking-tight tabular-nums">
                {formatUsd(totalEquity)}
              </Text>
            )}
          </View>
          <View className="flex flex-row items-center justify-between mt-2.5">
            <SparklinePlaceholder color="up" />
            <View className="flex flex-row gap-2">
              <Button variant="secondary" size="sm">
                <Text className="text-fg-primary text-[12px]">Deposit</Text>
              </Button>
              <Button variant="secondary" size="sm">
                <Text className="text-fg-primary text-[12px]">Withdraw</Text>
              </Button>
              <Button variant="primary" size="sm">
                <Text className="text-btn-primary-fg text-[12px]">Trade</Text>
              </Button>
            </View>
          </View>
        </KpiCard>

        <KpiCard label="Spot balance">
          {isLoading ? (
            <Skeleton height={32} width={120} className="mt-1.5" />
          ) : (
            <Text className="text-fg-primary font-medium text-[28px] tracking-tight tabular-nums mt-1.5">
              {formatUsd(spotTotal)}
            </Text>
          )}
          <Text className="text-fg-tertiary text-[12px] mt-2">{assets.length} assets</Text>
        </KpiCard>

        <KpiCard label="Margin used">
          <Text className="text-fg-tertiary font-medium text-[28px] tracking-tight tabular-nums mt-1.5">
            {"\u2014"}
          </Text>
          <View className="h-1 bg-bg-sunk rounded-full mt-3 overflow-hidden">
            <View className="h-full bg-accent rounded-full" style={{ width: "0%" }} />
          </View>
          <Text className="text-fg-tertiary text-[11px] mt-1.5">0% of equity</Text>
        </KpiCard>

        <KpiCard label="Unrealized PnL">
          <Text className="text-fg-tertiary font-medium text-[28px] tracking-tight tabular-nums mt-1.5">
            {"\u2014"}
          </Text>
          <Text className="text-fg-tertiary text-[12px] mt-2">0 open positions</Text>
        </KpiCard>
      </View>

      <HoldingsTable assets={assets} isLoading={isLoading} />
      <ActivityTable />
    </View>
  );
}
