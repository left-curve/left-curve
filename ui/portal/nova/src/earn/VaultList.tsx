import { useMemo } from "react";
import { View, Text } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Decimal } from "@left-curve/dango/utils";
import { useAccount, usePerpsVaultUserShares, perpsMarginAsset } from "@left-curve/store";
import { Badge, Button, Card, FormattedNumber, Skeleton, Table } from "../components";

type VaultRow = {
  readonly name: string;
  readonly symbol: string;
  readonly logoURI: string;
  readonly tvl: Decimal;
  readonly hasApy: boolean;
  readonly apy: Decimal;
  readonly positionValue: Decimal;
  readonly hasPosition: boolean;
};

const COL_WIDTHS = ["w-[200px]", "w-[140px]", "w-[120px]", "w-[140px]", "flex-1"] as const;

function VaultIcon({ symbol, logoURI }: { symbol: string; logoURI: string }) {
  if (logoURI) {
    return <img src={logoURI} alt={symbol} className="w-7 h-7 rounded-full" />;
  }
  return (
    <View className="w-7 h-7 rounded-full bg-bg-tint items-center justify-center">
      <Text className="text-fg-secondary text-[10px] font-semibold">{symbol[0]}</Text>
    </View>
  );
}

function VaultRowItem({ vault }: { vault: VaultRow }) {
  return (
    <Table.Row columns={COL_WIDTHS}>
      <Table.Cell index={0}>
        <View className="flex flex-row items-center gap-2.5">
          <VaultIcon symbol={vault.symbol} logoURI={vault.logoURI} />
          <View className="flex flex-col">
            <Text className="text-fg-primary text-[13px] font-medium">{vault.name}</Text>
            <Text className="text-fg-tertiary text-[11px]">{vault.symbol}</Text>
          </View>
        </View>
      </Table.Cell>

      <Table.Cell index={1}>
        <FormattedNumber
          value={vault.tvl.toString()}
          className="text-fg-primary text-[13px]"
          formatOptions={{ currency: "USD" }}
        />
      </Table.Cell>

      <Table.Cell index={2}>
        {vault.hasApy ? (
          <Badge variant="up">
            <FormattedNumber value={vault.apy.toString()} className="text-[11px]" colorSign />
          </Badge>
        ) : (
          <Text className="text-fg-quaternary text-[13px]">--</Text>
        )}
      </Table.Cell>

      <Table.Cell index={3}>
        {vault.hasPosition ? (
          <FormattedNumber
            value={vault.positionValue.toString()}
            className="text-fg-primary text-[13px] font-medium"
            formatOptions={{ currency: "USD" }}
          />
        ) : (
          <Text className="text-fg-quaternary text-[13px]">{"\u2014"}</Text>
        )}
      </Table.Cell>

      <Table.Cell index={4}>
        <View className="flex flex-row justify-end gap-1.5">
          <Button variant="primary" size="sm">
            <Text className="text-btn-primary-fg text-[12px]">Deposit</Text>
          </Button>
          <Button variant="ghost" size="sm" disabled={!vault.hasPosition}>
            <Text className="text-fg-secondary text-[12px]">Withdraw</Text>
          </Button>
        </View>
      </Table.Cell>
    </Table.Row>
  );
}

function VaultTableSkeleton() {
  return (
    <Card className="overflow-hidden">
      <View
        className={twMerge("flex flex-row items-center h-9 px-4", "border-b border-border-subtle")}
      >
        <Skeleton width={60} height={10} className="mr-auto" />
      </View>
      {Array.from({ length: 2 }, (_, i) => (
        <View
          key={`skeleton-row-${i}`}
          className="flex flex-row items-center h-11 px-4 border-b border-border-subtle gap-4"
        >
          <View className="flex flex-row items-center gap-2.5 w-[200px]">
            <Skeleton width={28} height={28} rounded />
            <View className="flex flex-col gap-1">
              <Skeleton width={80} height={12} />
              <Skeleton width={32} height={10} />
            </View>
          </View>
          <Skeleton width={60} height={12} />
          <Skeleton width={48} height={18} />
          <Skeleton width={60} height={12} />
          <View className="flex-1" />
        </View>
      ))}
    </Card>
  );
}

function useVaultRows(): { vaults: readonly VaultRow[]; isLoading: boolean } {
  const { vaultState, userVaultShares, userSharesValue } = usePerpsVaultUserShares();

  const vaults: readonly VaultRow[] = useMemo(() => {
    if (!vaultState) return [];

    const tvl = Decimal(vaultState.equity ?? "0");
    const hasPosition = userVaultShares !== "0";
    const positionValue = Decimal(userSharesValue || "0");

    return [
      {
        name: "Perps Vault",
        symbol: perpsMarginAsset.symbol,
        logoURI: perpsMarginAsset.logoURI,
        tvl,
        hasApy: false,
        apy: Decimal("0"),
        positionValue,
        hasPosition,
      },
    ];
  }, [vaultState, userVaultShares, userSharesValue]);

  return { vaults, isLoading: !vaultState };
}

export function VaultList() {
  const { account } = useAccount();
  const { vaults, isLoading } = useVaultRows();
  const isConnected = !!account;

  if (isLoading) {
    return <VaultTableSkeleton />;
  }

  if (vaults.length === 0) {
    return (
      <Card className="overflow-hidden">
        <Table.Empty>No vaults available</Table.Empty>
      </Card>
    );
  }

  return (
    <Card className="overflow-hidden">
      <Table>
        <Table.Header columns={COL_WIDTHS}>
          <Table.HeaderCell index={0}>Vault</Table.HeaderCell>
          <Table.HeaderCell index={1}>TVL</Table.HeaderCell>
          <Table.HeaderCell index={2}>APY</Table.HeaderCell>
          <Table.HeaderCell index={3}>
            {isConnected ? "Your Position" : "Position"}
          </Table.HeaderCell>
          <Table.HeaderCell index={4}>
            <Text />
          </Table.HeaderCell>
        </Table.Header>

        {!isConnected && (
          <View className="py-2 px-4">
            <Text className="text-fg-tertiary text-[11px]">
              Connect wallet to see your positions
            </Text>
          </View>
        )}

        {vaults.map((vault) => (
          <VaultRowItem key={vault.name} vault={vault} />
        ))}
      </Table>
    </Card>
  );
}
