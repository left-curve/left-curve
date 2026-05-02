import { useState, useCallback, useMemo } from "react";
import { View, Text, Pressable } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { useExplorerTransaction } from "@left-curve/store";
import { Card, Badge, Chip, Skeleton } from "../components";
import { primaryMethodName } from "./utils";

type InfoFieldProps = {
  readonly label: string;
  readonly value: string;
  readonly mono?: boolean;
  readonly copyable?: boolean;
};

function InfoField({ label, value, mono = false, copyable = false }: InfoFieldProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }, []);

  return (
    <View className="flex flex-col gap-1">
      <Text className="text-fg-tertiary text-[10px] font-semibold tracking-[var(--tracking-caps)] uppercase">
        {label}
      </Text>
      <View className="flex flex-row items-center gap-1.5">
        <Text
          className={twMerge("text-fg-primary text-[12px]", mono && "font-mono tabular-nums")}
          numberOfLines={1}
        >
          {value}
        </Text>
        {copyable && (
          <Pressable onPress={handleCopy}>
            <Text className={twMerge("text-[11px]", copied ? "text-up" : "text-fg-tertiary")}>
              {copied ? "\u2713" : "\u2398"}
            </Text>
          </Pressable>
        )}
      </View>
    </View>
  );
}

function TxDetailSkeleton() {
  return (
    <View className="flex-1 max-w-[1640px] mx-auto w-full p-4">
      <View className="flex flex-col gap-6">
        <View className="flex flex-col gap-2">
          <Skeleton height={32} width={200} />
          <Skeleton height={18} width="50%" />
        </View>
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

export function TxDetail({
  txHash,
  onBlockPress,
  onBack,
}: {
  txHash: string;
  onBlockPress?: (height: number) => void;
  onBack?: () => void;
}) {
  const [rawExpanded, setRawExpanded] = useState(false);
  const { data: tx, isLoading } = useExplorerTransaction(txHash);
  const [hashCopied, setHashCopied] = useState(false);

  const handleCopyHash = useCallback(() => {
    setHashCopied(true);
    setTimeout(() => setHashCopied(false), 1200);
  }, []);

  const rawData = useMemo(() => {
    if (!tx) return "";
    return JSON.stringify(
      {
        messages: tx.messages.map((msg) => ({
          method: msg.methodName,
          contract: msg.contractAddr,
          data: msg.data,
        })),
        gasUsed: tx.gasUsed,
        gasWanted: tx.gasWanted,
        nestedEvents: tx.nestedEvents,
      },
      null,
      2,
    );
  }, [tx]);

  if (isLoading) {
    return <TxDetailSkeleton />;
  }

  if (!tx) {
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
            Transaction not found
          </Text>
          <Text className="text-fg-secondary text-[12px] font-mono">{txHash}</Text>
        </View>
      </View>
    );
  }

  const method = primaryMethodName(tx);
  const timestamp = new Date(tx.createdAt).toLocaleString();

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

          <View className="flex flex-row items-center gap-3 flex-wrap">
            <Text className="text-fg-primary font-display text-[28px] font-semibold tracking-tight">
              Transaction
            </Text>
            <Badge variant={tx.hasSucceeded ? "up" : "down"}>
              <Text className="text-[11px] font-medium">
                {tx.hasSucceeded ? "Success" : "Failed"}
              </Text>
            </Badge>
          </View>

          <View className="flex flex-row items-center gap-2">
            <Text className="text-fg-secondary text-[12px] font-mono tabular-nums">{tx.hash}</Text>
            <Pressable onPress={handleCopyHash}>
              <Text className={twMerge("text-[11px]", hashCopied ? "text-up" : "text-fg-tertiary")}>
                {hashCopied ? "\u2713 Copied" : "\u2398 Copy"}
              </Text>
            </Pressable>
          </View>
        </View>

        <View
          style={{
            display: "grid" as never,
            gridTemplateColumns: "repeat(3, 1fr)",
            gap: 12,
          }}
        >
          <Card className="p-4 flex flex-col gap-3">
            <Pressable onPress={() => onBlockPress?.(tx.blockHeight)}>
              <InfoField label="Block" value={`#${tx.blockHeight.toLocaleString()}`} mono />
            </Pressable>
            <InfoField label="Timestamp" value={timestamp} />
          </Card>

          <Card className="p-4 flex flex-col gap-3">
            <InfoField label="Sender" value={tx.sender} mono copyable />
            <InfoField label="Transaction Index" value={String(tx.transactionIdx)} mono />
          </Card>

          <Card className="p-4 flex flex-col gap-3">
            <InfoField label="Gas Used" value={String(tx.gasUsed)} mono />
            <InfoField label="Gas Wanted" value={String(tx.gasWanted)} mono />
            <View className="flex flex-row items-center gap-2">
              <Text className="text-fg-tertiary text-[10px] font-semibold tracking-[var(--tracking-caps)] uppercase">
                Method
              </Text>
              <Chip variant="default" className="h-5 px-1.5 text-[10px] font-mono font-medium">
                <Text className="text-fg-primary text-[10px]">{method}</Text>
              </Chip>
            </View>
          </Card>
        </View>

        {tx.messages.length > 0 && (
          <Card className="p-0 overflow-hidden">
            <View className="px-4 py-3 border-b border-border-subtle bg-bg-sunk">
              <Text className="text-fg-primary font-display text-[13px] font-medium">
                Messages ({tx.messages.length})
              </Text>
            </View>
            {tx.messages.map((msg) => (
              <View key={msg.orderIdx} className="px-4 py-3 border-b border-border-subtle">
                <View className="flex flex-row items-center gap-2 mb-2">
                  <Chip variant="accent" className="h-5 px-1.5 text-[10px] font-mono font-medium">
                    <Text className="text-accent text-[10px]">{msg.methodName}</Text>
                  </Chip>
                  <Text className="text-fg-tertiary text-[11px] font-mono">{msg.contractAddr}</Text>
                </View>
                <Text
                  className="text-fg-secondary text-[11px] font-mono leading-relaxed"
                  style={{ whiteSpace: "pre-wrap" as never }}
                >
                  {JSON.stringify(msg.data, null, 2)}
                </Text>
              </View>
            ))}
          </Card>
        )}

        {!tx.hasSucceeded && tx.errorMessage && (
          <Card className="p-4 flex flex-col gap-2 border-down">
            <Text className="text-down text-[12px] font-semibold">Error</Text>
            <Text
              className="text-fg-secondary text-[11px] font-mono leading-relaxed"
              style={{ whiteSpace: "pre-wrap" as never }}
            >
              {tx.errorMessage}
            </Text>
          </Card>
        )}

        <Card className="p-0 overflow-hidden">
          <Pressable
            onPress={() => setRawExpanded((prev) => !prev)}
            className="px-4 py-3 flex flex-row items-center justify-between bg-bg-sunk border-b border-border-subtle"
          >
            <Text className="text-fg-primary font-display text-[13px] font-medium">Raw Data</Text>
            <Text className="text-fg-tertiary text-[12px]">
              {rawExpanded ? "\u25B2" : "\u25BC"}
            </Text>
          </Pressable>

          {rawExpanded && (
            <View className="p-4 bg-bg-sunk">
              <Text
                className="text-fg-secondary text-[11px] font-mono leading-relaxed"
                style={{ whiteSpace: "pre" as never }}
              >
                {rawData}
              </Text>
            </View>
          )}
        </Card>
      </View>
    </View>
  );
}
