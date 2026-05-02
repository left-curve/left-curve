import { View, Text } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Decimal, formatUnits } from "@left-curve/dango/utils";
import { useAccount, useBalances, useConfig, usePrices } from "@left-curve/store";
import { useMemo } from "react";
import { Button, Card, Skeleton, Tabs } from "../components";

type DecimalValue = ReturnType<typeof Decimal>;

type PortfolioAsset = {
  sym: string;
  name: string;
  denom: string;
  balance: DecimalValue;
  price: DecimalValue;
  value: DecimalValue;
  pct: DecimalValue;
};

const FILTER_TABS = [
  { value: "all", label: "All" },
  { value: "spot", label: "Spot" },
  { value: "perps", label: "Perps" },
] as const;

function formatUsd(value: DecimalValue, decimals = 2): string {
  const num = Number(value.toFixed(decimals));
  return `$${num.toLocaleString("en-US", { minimumFractionDigits: decimals, maximumFractionDigits: decimals })}`;
}

function formatBalance(value: DecimalValue): string {
  return Number(value.lt(Decimal("10")) ? value.toFixed(4) : value.toFixed(2)).toLocaleString(
    "en-US",
  );
}

const ALLOCATION_COLORS = [
  "bg-accent",
  "bg-up",
  "bg-btn-primary-bg",
  "bg-fg-tertiary",
  "bg-fg-quaternary",
];

function AllocationBar({ assets }: { assets: readonly PortfolioAsset[] }) {
  return (
    <View className="flex flex-col gap-3">
      <View className="h-2 rounded-full overflow-hidden flex flex-row">
        {assets.map((asset, i) => (
          <View
            key={asset.denom}
            className={twMerge("h-full", ALLOCATION_COLORS[i % ALLOCATION_COLORS.length])}
            style={{ width: `${Number(asset.pct.toFixed(1))}%` }}
          />
        ))}
      </View>
      <View className="flex flex-row flex-wrap gap-x-4 gap-y-1">
        {assets.map((asset, i) => (
          <View key={asset.denom} className="flex flex-row items-center gap-1.5">
            <View
              className={twMerge(
                "w-2 h-2 rounded-full",
                ALLOCATION_COLORS[i % ALLOCATION_COLORS.length],
              )}
            />
            <Text className="text-fg-secondary text-[12px]">{asset.sym}</Text>
            <Text className="text-fg-tertiary text-[12px] tabular-nums">
              {asset.pct.toFixed(1)}%
            </Text>
          </View>
        ))}
      </View>
    </View>
  );
}

function PortfolioRow({ asset }: { asset: PortfolioAsset }) {
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

      <View className="w-[80px]">
        <View className="flex flex-row items-center gap-1">
          <View className="flex-1 h-1 bg-bg-sunk rounded-full overflow-hidden">
            <View
              className="h-full bg-accent rounded-full"
              style={{ width: `${Number(asset.pct.toFixed(0))}%` }}
            />
          </View>
          <Text className="text-fg-tertiary text-[11px] tabular-nums">{asset.pct.toFixed(1)}%</Text>
        </View>
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

export function Portfolio() {
  const { account } = useAccount();
  const { coins } = useConfig();
  const { getPrice } = usePrices();
  const { data: balances = {}, isLoading } = useBalances({ address: account?.address });

  const { assets, totalValue } = useMemo(() => {
    const raw = Object.entries(balances)
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
          pct: Decimal("0"),
        };
      })
      .filter((a) => a.value.gt(Decimal("0")) || a.balance.gt(Decimal("0")))
      .sort((a, b) => Number(b.value.minus(a.value).toFixed(2)));

    const total = raw.reduce((sum, a) => sum.plus(a.value), Decimal("0"));

    const withPct = raw.map((a) => ({
      ...a,
      pct: total.gt(Decimal("0")) ? a.value.div(total).mul(Decimal("100")) : Decimal("0"),
    }));

    return { assets: withPct, totalValue: total };
  }, [balances, coins, getPrice]);

  return (
    <View className="flex flex-col gap-4">
      <View className="flex flex-row items-center justify-between">
        <Text className="text-fg-primary text-[20px] font-display font-semibold tracking-tight">
          Portfolio
        </Text>
      </View>

      <Card className="p-5">
        <View className="flex flex-row items-baseline gap-3 mb-4">
          {isLoading ? (
            <Skeleton height={32} width={180} />
          ) : (
            <Text className="text-fg-primary font-semibold text-[28px] tracking-tight tabular-nums">
              {formatUsd(totalValue)}
            </Text>
          )}
          <Text className="text-fg-tertiary text-[12px]">Total value</Text>
        </View>
        {isLoading ? (
          <Skeleton height={8} />
        ) : assets.length > 0 ? (
          <AllocationBar assets={assets} />
        ) : null}
      </Card>

      <Card className="overflow-hidden">
        <View className="flex flex-row items-center justify-between px-4 py-3">
          <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">Holdings</Text>
          <Tabs items={[...FILTER_TABS]} defaultValue="all" />
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
          <Text className="w-[80px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
            Alloc
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
            <Text className="text-fg-tertiary text-[13px]">No holdings</Text>
          </View>
        ) : (
          assets.map((asset) => <PortfolioRow key={asset.denom} asset={asset} />)
        )}
      </Card>
    </View>
  );
}
