import { useMemo } from "react";
import { View, Text, Pressable } from "react-native";
import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { Card, Table, Skeleton, FormattedNumber } from "../components";
import { formatTimeAgo, truncateHash } from "./utils";

import type { IndexedBlock } from "@left-curve/dango/types";

const BLOCK_COUNT = 10;
const POLL_INTERVAL = 6_000;

const COLUMNS = ["flex-[2]", "flex-[1.5]", "flex-1", "flex-[3]"] as const;

function BlockRowSkeleton() {
  return (
    <Table.Row columns={COLUMNS}>
      <Table.Cell index={0}>
        <Skeleton height={16} width={70} />
      </Table.Cell>
      <Table.Cell index={1}>
        <Skeleton height={16} width={50} />
      </Table.Cell>
      <Table.Cell index={2}>
        <Skeleton height={16} width={30} />
      </Table.Cell>
      <Table.Cell index={3}>
        <Skeleton height={16} width="80%" />
      </Table.Cell>
    </Table.Row>
  );
}

function BlockRow({
  block,
  onPress,
}: {
  readonly block: IndexedBlock;
  readonly onPress: () => void;
}) {
  return (
    <Table.Row columns={COLUMNS}>
      <Table.Cell index={0}>
        <Pressable onPress={onPress}>
          <FormattedNumber
            value={block.blockHeight}
            className="text-accent text-[12px] font-medium"
          />
        </Pressable>
      </Table.Cell>

      <Table.Cell index={1}>
        <Text className="text-fg-tertiary text-[12px] tabular-nums">
          {formatTimeAgo(block.createdAt)}
        </Text>
      </Table.Cell>

      <Table.Cell index={2}>
        <Text className="text-fg-secondary text-[12px] font-mono tabular-nums text-right">
          {block.transactions.length}
        </Text>
      </Table.Cell>

      <Table.Cell index={3}>
        <Text className="text-fg-secondary text-[12px] font-mono tabular-nums truncate text-right">
          {truncateHash(block.hash, 10, 0)}
        </Text>
      </Table.Cell>
    </Table.Row>
  );
}

export function LatestBlocks({
  onBlockPress,
}: {
  readonly onBlockPress?: (height: number) => void;
}) {
  const client = usePublicClient();

  const { data: latestBlock } = useQuery({
    queryKey: ["block_explorer", "latest"],
    queryFn: () => client.queryBlock(),
    refetchInterval: POLL_INTERVAL,
  });

  const blockHeights = useMemo(() => {
    if (!latestBlock) return [];
    const start = latestBlock.blockHeight;
    return Array.from({ length: BLOCK_COUNT }, (_, i) => start - i).filter((h) => h > 0);
  }, [latestBlock]);

  const { data: blocks, isLoading } = useQuery({
    queryKey: ["explorer_latest_blocks", blockHeights],
    queryFn: () => Promise.all(blockHeights.map((height) => client.queryBlock({ height }))),
    enabled: blockHeights.length > 0,
    refetchInterval: POLL_INTERVAL,
  });

  return (
    <Card className="p-0 overflow-hidden flex-1">
      <View className="px-4 py-3 border-b border-border-subtle bg-bg-sunk">
        <Text className="text-fg-primary font-display text-[13px] font-medium">Latest Blocks</Text>
      </View>

      <Table>
        <Table.Header columns={COLUMNS}>
          <Table.HeaderCell index={0}>Block</Table.HeaderCell>
          <Table.HeaderCell index={1}>Time</Table.HeaderCell>
          <Table.HeaderCell index={2}>Txns</Table.HeaderCell>
          <Table.HeaderCell index={3}>Hash</Table.HeaderCell>
        </Table.Header>

        {isLoading || !blocks
          ? Array.from({ length: BLOCK_COUNT }, (_, i) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: skeleton placeholders have no stable id
              <BlockRowSkeleton key={i} />
            ))
          : blocks.map((block) => (
              <BlockRow
                key={block.blockHeight}
                block={block}
                onPress={() => onBlockPress?.(block.blockHeight)}
              />
            ))}

        {!isLoading && blocks?.length === 0 && <Table.Empty>No blocks found</Table.Empty>}
      </Table>

      <View className="px-4 py-3 border-t border-border-subtle bg-bg-sunk items-center">
        <Pressable>
          <Text className="text-accent text-[12px] font-medium">View all blocks</Text>
        </Pressable>
      </View>
    </Card>
  );
}
