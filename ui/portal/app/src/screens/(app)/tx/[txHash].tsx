import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useRouter, useLocalSearchParams } from "expo-router";
import { parseExplorerErrorMessage, useExplorerTransaction } from "@left-curve/store";

import { View } from "react-native";
import { AddressVisualizer } from "~/components/foundation/AddressVisualizer";
import { Badge, GlobalText, MobileTitle, Skeleton } from "~/components/foundation";
import {
  ExplorerAccordion,
  ExplorerHashValue,
  ExplorerJsonBlock,
  ExplorerKeyValueRow,
  ExplorerNotFound,
  ExplorerScreen,
  ExplorerSectionCard,
} from "~/components/explorer/ExplorerCommon";

import type { Address } from "@left-curve/dango/types";

export default function TxExplorerScreen() {
  const { txHash } = useLocalSearchParams<{ txHash: string }>();
  const router = useRouter();
  const { data: tx, isLoading } = useExplorerTransaction(txHash || "");

  const { error, backtrace } = parseExplorerErrorMessage(tx?.errorMessage);

  return (
    <ExplorerScreen>
      <MobileTitle title={m["explorer.txs.title"]()} />

      {isLoading ? (
        <ExplorerSectionCard title={m["explorer.txs.txDetails"]()}>
          <Skeleton className="w-full h-10" />
          <Skeleton className="w-full h-10" />
          <Skeleton className="w-full h-10" />
        </ExplorerSectionCard>
      ) : null}

      {!isLoading && !tx ? (
        <ExplorerNotFound
          title={m["explorer.txs.notFound.title"]()}
          description={
            <>
              {m["explorer.txs.notFound.pre"]()} <GlobalText className="underline">{txHash}</GlobalText>{" "}
              {m["explorer.txs.notFound.description"]()}
            </>
          }
        />
      ) : null}

      {tx ? (
        <>
          <ExplorerSectionCard title={m["explorer.txs.txDetails"]()}>
            <ExplorerKeyValueRow label={m["explorer.txs.txHash"]()}>
              <ExplorerHashValue value={tx.hash} />
            </ExplorerKeyValueRow>

            <ExplorerKeyValueRow label={m["explorer.txs.sender"]()}>
              <AddressVisualizer
                withIcon
                address={tx.sender as Address}
                classNames={{ text: "diatype-sm-bold" }}
                onClick={(url) => router.push(url as never)}
              />
            </ExplorerKeyValueRow>

            <ExplorerKeyValueRow label={m["explorer.txs.time"]()}>
              <GlobalText>{new Date(tx.createdAt).toLocaleString()}</GlobalText>
            </ExplorerKeyValueRow>

            <ExplorerKeyValueRow label={m["explorer.txs.block"]()}>
              <GlobalText
                className="text-ink-secondary-blue diatype-sm-bold"
                onPress={() => router.push(`/block/${tx.blockHeight}` as never)}
              >
                {tx.blockHeight}
              </GlobalText>
            </ExplorerKeyValueRow>

            <ExplorerKeyValueRow label={m["explorer.txs.index"]()}>
              <GlobalText>{tx.transactionIdx}</GlobalText>
            </ExplorerKeyValueRow>

            <ExplorerKeyValueRow label={m["explorer.txs.gasUsed"]()}>
              <GlobalText>{tx.gasUsed}</GlobalText>
            </ExplorerKeyValueRow>

            <ExplorerKeyValueRow label={m["explorer.txs.gasWanted"]()}>
              <GlobalText>{tx.gasWanted}</GlobalText>
            </ExplorerKeyValueRow>

            <ExplorerKeyValueRow label={m["explorer.txs.status"]()}>
              <Badge
                color={tx.hasSucceeded ? "green" : "red"}
                text={tx.hasSucceeded ? m["explorer.txs.success"]() : m["explorer.txs.failed"]()}
              />
            </ExplorerKeyValueRow>
          </ExplorerSectionCard>

          {error || backtrace ? (
            <ExplorerSectionCard title={m["explorer.txs.error"]()}>
              {error ? (
                <ExplorerAccordion title={m["explorer.txs.message"]()}>
                  <ExplorerJsonBlock data={{ error }} />
                </ExplorerAccordion>
              ) : null}
              {backtrace ? (
                <ExplorerAccordion title={m["explorer.txs.backtrace"]()}>
                  <ExplorerJsonBlock data={backtrace.replace(/ (\d+):/g, "\n$1:")} />
                </ExplorerAccordion>
              ) : null}
            </ExplorerSectionCard>
          ) : null}

          <ExplorerSectionCard title={m["explorer.txs.messages"]()}>
            {tx.messages.map(({ data, methodName, orderIdx }) => (
              <View key={orderIdx}>
                <ExplorerAccordion title={methodName} defaultExpanded>
                  <ExplorerJsonBlock data={data[methodName]} />
                </ExplorerAccordion>
              </View>
            ))}
          </ExplorerSectionCard>

          <ExplorerSectionCard title={m["explorer.txs.events"]()}>
            <ExplorerJsonBlock data={tx.nestedEvents} />
          </ExplorerSectionCard>
        </>
      ) : null}
    </ExplorerScreen>
  );
}
