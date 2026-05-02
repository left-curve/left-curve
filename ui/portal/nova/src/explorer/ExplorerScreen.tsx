import { useState, useCallback } from "react";
import { View, Text, TextInput, Pressable } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { useNavigate } from "@tanstack/react-router";
import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { Card, Skeleton } from "../components";
import { LatestBlocks } from "./LatestBlocks";
import { LatestTransactions } from "./LatestTransactions";
import { truncateHash } from "./utils";

const POLL_INTERVAL = 6_000;

type KpiCardProps = {
  readonly label: string;
  readonly value: string | undefined;
  readonly isLoading: boolean;
};

function KpiCard({ label, value, isLoading }: KpiCardProps) {
  return (
    <Card className="flex-1 min-w-[200px] p-4 flex flex-col gap-1.5">
      <Text className="text-fg-tertiary text-[10px] font-semibold tracking-[var(--tracking-caps)] uppercase">
        {label}
      </Text>
      {isLoading ? (
        <Skeleton height={28} width="60%" />
      ) : (
        <Text className="text-fg-primary font-display text-[20px] font-semibold tracking-tight font-mono tabular-nums">
          {value ?? "-"}
        </Text>
      )}
    </Card>
  );
}

function SearchBar({
  value,
  onChangeText,
  onSubmit,
}: {
  readonly value: string;
  readonly onChangeText: (text: string) => void;
  readonly onSubmit: () => void;
}) {
  return (
    <View
      className={twMerge(
        "flex flex-row items-center",
        "h-11 px-3.5",
        "bg-bg-surface",
        "border border-border-default rounded-card",
        "hover:border-border-strong focus-within:border-fg-primary focus-within:bg-bg-elev",
        "transition-[border-color,background] duration-150 ease-[var(--ease)]",
      )}
    >
      <Text className="text-fg-tertiary text-[12px] mr-2.5 shrink-0">{"\u{1F50D}"}</Text>
      <TextInput
        className="flex-1 min-w-0 h-full bg-transparent border-0 outline-none text-[13px] text-fg-primary placeholder:text-fg-quaternary"
        placeholder="Search by tx hash, block #, or address..."
        value={value}
        onChangeText={onChangeText}
        onSubmitEditing={onSubmit}
      />
      <View className="px-1.5 py-0.5 border border-border-subtle rounded font-mono shrink-0">
        <Text className="text-fg-tertiary text-[10px] font-mono">Enter</Text>
      </View>
    </View>
  );
}

function resolveSearchTarget(query: string): string | undefined {
  const trimmed = query.trim();
  if (!trimmed) return undefined;

  const withoutHash = trimmed.startsWith("#") ? trimmed.slice(1) : trimmed;
  const asNumber = Number(withoutHash.replace(/,/g, ""));
  if (!Number.isNaN(asNumber) && asNumber > 0) {
    return `/explorer/block/${asNumber}`;
  }

  if (trimmed.startsWith("0x") || trimmed.length === 64) {
    return `/explorer/tx/${trimmed}`;
  }

  return undefined;
}

function QuickLinks() {
  const items = [
    { label: "tx hash", example: "0x6AE293...0CFD148" },
    { label: "address", example: "dango1qx7e...k4p2" },
    { label: "block", example: "#12,847,400" },
  ] as const;

  return (
    <View className="flex flex-row gap-1.5 flex-wrap">
      <Text className="text-fg-tertiary text-[11px] py-0.5 mr-1">Try:</Text>
      {items.map(({ label, example }) => (
        <Pressable
          key={label}
          className="inline-flex flex-row items-center h-6 px-2 border border-border-subtle rounded-chip bg-transparent hover:bg-bg-tint transition-[background] duration-150 ease-[var(--ease)]"
        >
          <Text className="text-fg-tertiary text-[11px] mr-1.5">{label}</Text>
          <Text className="text-fg-secondary text-[11px] font-mono">{example}</Text>
        </Pressable>
      ))}
    </View>
  );
}

export function ExplorerScreen() {
  const [searchQuery, setSearchQuery] = useState("");
  const navigate = useNavigate();
  const client = usePublicClient();

  const { data: latestBlock, isLoading } = useQuery({
    queryKey: ["block_explorer", "latest"],
    queryFn: () => client.queryBlock(),
    refetchInterval: POLL_INTERVAL,
  });

  const handleSearch = useCallback(() => {
    const target = resolveSearchTarget(searchQuery);
    if (target) {
      navigate({ to: target });
    }
  }, [searchQuery, navigate]);

  const handleBlockPress = useCallback(
    (height: number) => navigate({ to: `/explorer/block/${height}` }),
    [navigate],
  );

  const handleTxPress = useCallback(
    (hash: string) => navigate({ to: `/explorer/tx/${hash}` }),
    [navigate],
  );

  const blockHashDisplay = latestBlock?.hash ? truncateHash(latestBlock.hash, 12, 0) : undefined;

  const blockTimeDisplay = latestBlock?.createdAt
    ? new Date(latestBlock.createdAt).toLocaleTimeString()
    : undefined;

  return (
    <View className="flex-1 max-w-[1640px] mx-auto w-full p-4">
      <View className="flex flex-col gap-6">
        <View className="flex flex-col gap-2">
          <Text className="text-fg-primary font-display text-[28px] font-semibold tracking-tight">
            Explorer
          </Text>
          <Text className="text-fg-secondary text-[12px] leading-relaxed">
            Browse blocks, transactions, and accounts on the Dango network.
          </Text>
        </View>

        <View className="flex flex-col gap-2">
          <SearchBar value={searchQuery} onChangeText={setSearchQuery} onSubmit={handleSearch} />
          <QuickLinks />
        </View>

        <View className="flex flex-row gap-3 flex-wrap">
          <KpiCard
            label="Latest Block"
            value={latestBlock ? latestBlock.blockHeight.toLocaleString() : undefined}
            isLoading={isLoading}
          />
          <KpiCard
            label="Transactions in Block"
            value={latestBlock ? String(latestBlock.transactions.length) : undefined}
            isLoading={isLoading}
          />
          <KpiCard label="Block Hash" value={blockHashDisplay} isLoading={isLoading} />
          <KpiCard label="Block Time" value={blockTimeDisplay} isLoading={isLoading} />
        </View>

        <View
          style={{
            display: "grid" as never,
            gridTemplateColumns: "1fr 1fr",
            gap: 12,
          }}
        >
          <LatestBlocks onBlockPress={handleBlockPress} />
          <LatestTransactions onTxPress={handleTxPress} />
        </View>
      </View>
    </View>
  );
}
