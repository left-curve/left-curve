import { View, Text, Pressable } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { useExplorerBlock } from "@left-curve/store";
import { Card, Badge, Chip, Skeleton } from "../components";
import { truncateHash, primaryMethodName } from "./utils";

import type { IndexedTransaction } from "@left-curve/dango/types";

type InfoFieldProps = {
  readonly label: string;
  readonly value: string;
  readonly mono?: boolean;
};

function InfoField({ label, value, mono = false }: InfoFieldProps) {
  return (
    <View className="flex flex-col gap-1">
      <Text className="text-fg-tertiary text-[10px] font-semibold tracking-[var(--tracking-caps)] uppercase">
        {label}
      </Text>
      <Text className={twMerge("text-fg-primary text-[12px]", mono && "font-mono tabular-nums")}>
        {value}
      </Text>
    </View>
  );
}

function BlockTxRow({
  tx,
  isLast,
  onTxPress,
}: {
  tx: IndexedTransaction;
  isLast: boolean;
  onTxPress?: (hash: string) => void;
}) {
  const method = primaryMethodName(tx);

  return (
    <View
      className={twMerge(
        "flex flex-row items-center px-4 py-2.5",
        !isLast && "border-b border-border-subtle",
      )}
      style={{
        display: "grid" as never,
        gridTemplateColumns: "140px 100px 140px 80px",
        gap: 8,
      }}
    >
      <Pressable onPress={() => onTxPress?.(tx.hash)}>
        <Text className="text-accent font-mono text-[12px] tabular-nums font-medium truncate">
          {truncateHash(tx.hash)}
        </Text>
      </Pressable>

      <Chip
        variant={tx.hasSucceeded ? "default" : "down"}
        className="h-5 px-1.5 text-[10px] font-mono font-medium"
      >
        <Text className={`text-[10px] ${tx.hasSucceeded ? "text-fg-primary" : "text-down"}`}>
          {method}
        </Text>
      </Chip>

      <Text className="text-fg-secondary text-[12px] font-mono tabular-nums truncate">
        {truncateHash(tx.sender)}
      </Text>

      <View className="items-end">
        <Badge variant={tx.hasSucceeded ? "up" : "down"}>
          <Text className="text-[10px] font-medium">{tx.hasSucceeded ? "Success" : "Failed"}</Text>
        </Badge>
      </View>
    </View>
  );
}

function BlockDetailSkeleton() {
  return (
    <View className="flex-1 max-w-[1640px] mx-auto w-full p-4">
      <View className="flex flex-col gap-6">
        <Skeleton height={32} width={200} />
        <View
          style={{
            display: "grid" as never,
            gridTemplateColumns: "repeat(3, 1fr)",
            gap: 12,
          }}
        >
          {Array.from({ length: 3 }, (_, i) => (
            <Card key={i} className="p-4 flex flex-col gap-3">
              <Skeleton height={14} width="40%" />
              <Skeleton height={18} width="70%" />
              <Skeleton height={14} width="40%" />
              <Skeleton height={18} width="60%" />
            </Card>
          ))}
        </View>
      </View>
    </View>
  );
}

export function BlockDetail({
  blockHeight,
  onTxPress,
  onBack,
}: {
  blockHeight: number;
  onTxPress?: (hash: string) => void;
  onBack?: () => void;
}) {
  const { data, isLoading } = useExplorerBlock(String(blockHeight));

  if (isLoading) {
    return <BlockDetailSkeleton />;
  }

  const block = data?.searchBlock;

  if (!block) {
    return (
      <View className="flex-1 max-w-[1640px] mx-auto w-full p-4">
        <View className="flex flex-col gap-4">
          {onBack && (
            <Pressable onPress={onBack} className="flex flex-row items-center gap-1 mb-1">
              <Text className="text-accent text-[12px] font-medium">
                {"\u2190"} Back to Explorer
              </Text>
            </Pressable>
          )}
          <Text className="text-fg-primary font-display text-[20px] font-semibold">
            Block #{blockHeight.toLocaleString()} not found
          </Text>
        </View>
      </View>
    );
  }

  const txs = block.transactions;
  const timestamp = new Date(block.createdAt).toLocaleString();

  return (
    <View className="flex-1 max-w-[1640px] mx-auto w-full p-4">
      <View className="flex flex-col gap-6">
        <View className="flex flex-col gap-2">
          {onBack && (
            <Pressable onPress={onBack} className="flex flex-row items-center gap-1 mb-1">
              <Text className="text-accent text-[12px] font-medium">
                {"\u2190"} Back to Explorer
              </Text>
            </Pressable>
          )}
          <Text className="text-fg-primary font-display text-[28px] font-semibold tracking-tight">
            Block{" "}
            <Text className="font-mono tabular-nums">#{block.blockHeight.toLocaleString()}</Text>
          </Text>
        </View>

        <View
          style={{
            display: "grid" as never,
            gridTemplateColumns: "repeat(3, 1fr)",
            gap: 12,
          }}
        >
          <Card className="p-4 flex flex-col gap-3">
            <InfoField label="Height" value={block.blockHeight.toLocaleString()} mono />
            <InfoField label="Timestamp" value={timestamp} />
          </Card>
          <Card className="p-4 flex flex-col gap-3">
            <InfoField label="Block Hash" value={block.hash} mono />
            <InfoField label="App Hash" value={block.appHash} mono />
          </Card>
          <Card className="p-4 flex flex-col gap-3">
            <InfoField label="Transactions" value={String(txs.length)} mono />
          </Card>
        </View>

        <Card className="p-0 overflow-hidden">
          <View className="px-4 py-3 border-b border-border-subtle bg-bg-sunk flex flex-row items-center justify-between">
            <Text className="text-fg-primary font-display text-[13px] font-medium">
              Transactions in Block
            </Text>
            <Text className="text-fg-tertiary text-[12px] font-mono tabular-nums">
              {txs.length} txns
            </Text>
          </View>

          <View
            className="px-4 py-2.5 border-b border-border-subtle bg-bg-sunk"
            style={{
              display: "grid" as never,
              gridTemplateColumns: "140px 100px 140px 80px",
              gap: 8,
            }}
          >
            <Text className="text-fg-tertiary text-[10px] font-semibold tracking-[var(--tracking-wide)] uppercase">
              Tx Hash
            </Text>
            <Text className="text-fg-tertiary text-[10px] font-semibold tracking-[var(--tracking-wide)] uppercase">
              Method
            </Text>
            <Text className="text-fg-tertiary text-[10px] font-semibold tracking-[var(--tracking-wide)] uppercase">
              Sender
            </Text>
            <Text className="text-fg-tertiary text-[10px] font-semibold tracking-[var(--tracking-wide)] uppercase text-right">
              Status
            </Text>
          </View>

          {txs.length > 0 ? (
            txs.map((tx, i) => (
              <BlockTxRow
                key={tx.hash}
                tx={tx}
                isLast={i === txs.length - 1}
                onTxPress={onTxPress}
              />
            ))
          ) : (
            <View className="px-4 py-8 items-center">
              <Text className="text-fg-tertiary text-[12px]">No transactions in this block</Text>
            </View>
          )}
        </Card>
      </View>
    </View>
  );
}
