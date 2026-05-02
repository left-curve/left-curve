import { useMemo } from "react";
import { View, Text, Pressable } from "react-native";
import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { Card, Chip, Table, Skeleton, FormattedNumber } from "../components";
import { formatTimeAgo, truncateHash, primaryMethodName } from "./utils";

import type { IndexedTransaction } from "@left-curve/dango/types";

const BLOCK_SCAN_COUNT = 10;
const TX_DISPLAY_COUNT = 10;
const POLL_INTERVAL = 6_000;

const COLUMNS = ["flex-[2.5]", "flex-[1.5]", "flex-[2.5]", "flex-[1.5]", "flex-1"] as const;

function TxRowSkeleton() {
  return (
    <Table.Row columns={COLUMNS}>
      <Table.Cell index={0}>
        <Skeleton height={16} width={100} />
      </Table.Cell>
      <Table.Cell index={1}>
        <Skeleton height={20} width={80} />
      </Table.Cell>
      <Table.Cell index={2}>
        <Skeleton height={16} width={90} />
      </Table.Cell>
      <Table.Cell index={3}>
        <Skeleton height={16} width={50} />
      </Table.Cell>
      <Table.Cell index={4}>
        <Skeleton height={16} width={40} />
      </Table.Cell>
    </Table.Row>
  );
}

function TxRow({ tx, onPress }: { readonly tx: IndexedTransaction; readonly onPress: () => void }) {
  const method = primaryMethodName(tx);

  return (
    <Table.Row columns={COLUMNS}>
      <Table.Cell index={0}>
        <Pressable onPress={onPress}>
          <Text className="text-accent font-mono text-[12px] tabular-nums font-medium truncate">
            {truncateHash(tx.hash)}
          </Text>
        </Pressable>
      </Table.Cell>

      <Table.Cell index={1}>
        <Chip
          variant={tx.hasSucceeded ? "default" : "down"}
          className="h-5 px-1.5 text-[10px] font-mono font-medium"
        >
          <Text className={`text-[10px] ${tx.hasSucceeded ? "text-fg-primary" : "text-down"}`}>
            {method}
          </Text>
        </Chip>
      </Table.Cell>

      <Table.Cell index={2}>
        <Text className="text-fg-secondary text-[12px] font-mono tabular-nums truncate">
          {truncateHash(tx.sender)}
        </Text>
      </Table.Cell>

      <Table.Cell index={3}>
        <Text className="text-fg-secondary text-[12px] font-mono tabular-nums text-right">
          <FormattedNumber
            value={tx.blockHeight}
            className="text-fg-secondary text-[12px] font-mono tabular-nums"
          />
        </Text>
      </Table.Cell>

      <Table.Cell index={4}>
        <Text className="text-fg-tertiary text-[12px] tabular-nums text-right">
          {formatTimeAgo(tx.createdAt)}
        </Text>
      </Table.Cell>
    </Table.Row>
  );
}

export function LatestTransactions({ onTxPress }: { readonly onTxPress?: (hash: string) => void }) {
  const client = usePublicClient();

  const { data: latestBlock } = useQuery({
    queryKey: ["block_explorer", "latest"],
    queryFn: () => client.queryBlock(),
    refetchInterval: POLL_INTERVAL,
  });

  const blockHeights = useMemo(() => {
    if (!latestBlock) return [];
    const start = latestBlock.blockHeight;
    return Array.from({ length: BLOCK_SCAN_COUNT }, (_, i) => start - i).filter((h) => h > 0);
  }, [latestBlock]);

  const { data: blocks, isLoading } = useQuery({
    queryKey: ["explorer_latest_blocks", blockHeights],
    queryFn: () => Promise.all(blockHeights.map((height) => client.queryBlock({ height }))),
    enabled: blockHeights.length > 0,
    refetchInterval: POLL_INTERVAL,
  });

  const txs = useMemo(() => {
    if (!blocks) return [];
    return blocks.flatMap((block) => block.transactions).slice(0, TX_DISPLAY_COUNT);
  }, [blocks]);

  return (
    <Card className="p-0 overflow-hidden flex-1">
      <View className="px-4 py-3 border-b border-border-subtle bg-bg-sunk">
        <Text className="text-fg-primary font-display text-[13px] font-medium">
          Latest Transactions
        </Text>
      </View>

      <Table>
        <Table.Header columns={COLUMNS}>
          <Table.HeaderCell index={0}>Tx Hash</Table.HeaderCell>
          <Table.HeaderCell index={1}>Method</Table.HeaderCell>
          <Table.HeaderCell index={2}>Sender</Table.HeaderCell>
          <Table.HeaderCell index={3}>Block</Table.HeaderCell>
          <Table.HeaderCell index={4}>Time</Table.HeaderCell>
        </Table.Header>

        {isLoading || !blocks ? (
          Array.from({ length: TX_DISPLAY_COUNT }, (_, i) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: skeleton placeholders have no stable id
            <TxRowSkeleton key={i} />
          ))
        ) : txs.length === 0 ? (
          <Table.Empty>No recent transactions</Table.Empty>
        ) : (
          txs.map((tx) => <TxRow key={tx.hash} tx={tx} onPress={() => onTxPress?.(tx.hash)} />)
        )}
      </Table>

      <View className="px-4 py-3 border-t border-border-subtle bg-bg-sunk items-center">
        <Pressable>
          <Text className="text-accent text-[12px] font-medium">View all transactions</Text>
        </Pressable>
      </View>
    </Card>
  );
}
