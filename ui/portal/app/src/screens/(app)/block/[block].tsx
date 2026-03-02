import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useLocalSearchParams, useRouter } from "expo-router";
import { useEffect, useMemo, useState } from "react";
import { useExplorerBlock } from "@left-curve/store";

import { GlobalText, MobileTitle, Skeleton } from "~/components/foundation";
import {
  ExplorerHashValue,
  ExplorerJsonBlock,
  ExplorerKeyValueRow,
  ExplorerNotFound,
  ExplorerScreen,
  ExplorerSectionCard,
  ExplorerTransactionsList,
} from "~/components/explorer/ExplorerCommon";

export default function BlockExplorerScreen() {
  const { block } = useLocalSearchParams<{ block: string }>();
  const router = useRouter();
  const { data, isLoading } = useExplorerBlock(block || "");
  const [estimatedDateMs, setEstimatedDateMs] = useState<number>();
  const [nowMs, setNowMs] = useState<number>(Date.now());

  useEffect(() => {
    if (!data?.isFutureBlock) return;

    const blockDiff = data.height - data.currentBlock.blockHeight;
    setEstimatedDateMs(Date.now() + blockDiff * 500);
  }, [data]);

  useEffect(() => {
    if (!data?.isFutureBlock) return;
    const id = setInterval(() => setNowMs(Date.now()), 1000);
    return () => clearInterval(id);
  }, [data?.isFutureBlock]);

  const remainingBlocks = useMemo(() => {
    if (!data?.isFutureBlock || !estimatedDateMs) return 0;
    const msLeft = Math.max(0, estimatedDateMs - nowMs);
    return Math.ceil(msLeft / 500);
  }, [data?.isFutureBlock, estimatedDateMs, nowMs]);

  const secondsLeft = useMemo(() => {
    if (!estimatedDateMs) return 0;
    return Math.max(0, Math.ceil((estimatedDateMs - nowMs) / 1000));
  }, [estimatedDateMs, nowMs]);

  const cronOutcomes = useMemo(() => {
    if (!data?.searchBlock?.cronsOutcomes) return [];

    try {
      const parsed = JSON.parse(data.searchBlock.cronsOutcomes);
      return Array.isArray(parsed) ? parsed : [parsed];
    } catch {
      return [];
    }
  }, [data?.searchBlock?.cronsOutcomes]);

  return (
    <ExplorerScreen>
      <MobileTitle title={m["explorer.block.title"]()} />

      {isLoading ? (
        <ExplorerSectionCard title={m["explorer.block.details.blockDetails"]({ height: "#" })}>
          <Skeleton className="w-full h-10" />
          <Skeleton className="w-full h-10" />
          <Skeleton className="w-full h-10" />
        </ExplorerSectionCard>
      ) : null}

      {data?.isInvalidBlock ? (
        <ExplorerNotFound
          title={m["explorer.block.notFound.title"]()}
          description={m["explorer.block.notFound.description"]()}
        />
      ) : null}

      {!isLoading && data && !data.searchBlock && !data.isFutureBlock && !data.isInvalidBlock ? (
        <ExplorerNotFound
          title={m["explorer.block.notFound.title"]()}
          description={m["explorer.block.notFound.description"]()}
        />
      ) : null}

      {data?.isFutureBlock ? (
        <ExplorerSectionCard title={`${m["explorer.block.futureBlock.targetBlock"]()} ${data.height}`}>
          <ExplorerKeyValueRow label={m["explorer.block.futureBlock.currentBlock"]()}>
            <GlobalText>#{data.currentBlock.blockHeight}</GlobalText>
          </ExplorerKeyValueRow>
          <ExplorerKeyValueRow label={m["explorer.block.futureBlock.remainingBlocks"]()}>
            <GlobalText>#{remainingBlocks}</GlobalText>
          </ExplorerKeyValueRow>
          <ExplorerKeyValueRow label={m["countdown.seconds"]({ seconds: secondsLeft })}>
            <GlobalText>{secondsLeft}</GlobalText>
          </ExplorerKeyValueRow>
          <ExplorerKeyValueRow label={m["explorer.block.futureBlock.estimateTimeISO"]()}>
            <GlobalText>{estimatedDateMs ? new Date(estimatedDateMs).toISOString() : "-"}</GlobalText>
          </ExplorerKeyValueRow>
          <ExplorerKeyValueRow label={m["explorer.block.futureBlock.estimateTimeUTC"]()}>
            <GlobalText>{estimatedDateMs ? new Date(estimatedDateMs).toUTCString() : "-"}</GlobalText>
          </ExplorerKeyValueRow>
        </ExplorerSectionCard>
      ) : null}

      {data?.searchBlock ? (
        <>
          <ExplorerSectionCard
            title={m["explorer.block.details.blockDetails"]({ height: `#${data.searchBlock.blockHeight}` })}
          >
            <ExplorerKeyValueRow label={m["explorer.block.details.blockHash"]()}>
              <ExplorerHashValue value={data.searchBlock.hash} />
            </ExplorerKeyValueRow>
            <ExplorerKeyValueRow label={m["explorer.block.details.proposer"]()}>
              <GlobalText>Leftcurve Validator</GlobalText>
            </ExplorerKeyValueRow>
            <ExplorerKeyValueRow label={m["explorer.block.details.numberOfTx"]()}>
              <GlobalText>{data.searchBlock.transactions.length}</GlobalText>
            </ExplorerKeyValueRow>
            <ExplorerKeyValueRow label={m["explorer.block.details.blockTime"]()}>
              <GlobalText>{new Date(data.searchBlock.createdAt).toLocaleString()}</GlobalText>
            </ExplorerKeyValueRow>
          </ExplorerSectionCard>

          {cronOutcomes.length ? (
            <ExplorerSectionCard title={m["explorer.block.cronsOutcomes"]()}>
              <ExplorerJsonBlock data={cronOutcomes} />
            </ExplorerSectionCard>
          ) : null}

          <ExplorerTransactionsList
            transactions={data.searchBlock.transactions}
            onOpenTx={(hash) => router.push(`/tx/${hash}` as never)}
            onOpenBlock={(height) => router.push(`/block/${height}` as never)}
            onOpenAddress={(url) => router.push(url as never)}
          />
        </>
      ) : null}
    </ExplorerScreen>
  );
}
