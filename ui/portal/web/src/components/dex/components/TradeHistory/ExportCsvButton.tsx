import { Button, Spinner } from "@left-curve/applets-kit";
import { useAccount, usePublicClient } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { PerpsEvent } from "@left-curve/types";
import { useCallback, useMemo, useState } from "react";

import { buildPerpsTradeHistoryCsv, downloadCsv, tradeHistoryCsvFilename } from "./exportCsv";
import type { QueryRange } from "./useTradeHistoryFilter";

import type { PublicClient } from "@left-curve/sdk";

export const EXPORT_PAGE_SIZE = 100;
export const EXPORT_PAGE_DELAY_MS = 1000;

type FetchAllPerpsTradeHistoryParameters = {
  address: string;
  client: Pick<PublicClient, "queryPerpsEvents">;
  onProgress?: (count: number) => void;
  queryRange: QueryRange;
  wait?: (ms: number) => Promise<void>;
};

const waitForNextExportPage = (ms: number) =>
  new Promise<void>((resolve) => setTimeout(resolve, ms));

export async function fetchAllPerpsTradeHistory({
  address,
  client,
  onProgress,
  queryRange,
  wait = waitForNextExportPage,
}: FetchAllPerpsTradeHistoryParameters): Promise<PerpsEvent[]> {
  const events: PerpsEvent[] = [];
  let after: string | undefined;

  while (true) {
    const page = await client.queryPerpsEvents({
      userAddr: address,
      sortBy: "BLOCK_HEIGHT_DESC",
      earlierThan: queryRange.earlierThan,
      laterThan: queryRange.laterThan,
      first: EXPORT_PAGE_SIZE,
      after,
    });

    events.push(...page.nodes);
    onProgress?.(events.length);

    if (!page.pageInfo.hasNextPage) return events;

    after = page.pageInfo.endCursor ?? undefined;
    if (!after) throw new Error("Missing cursor for next trade history export page");

    await wait(EXPORT_PAGE_DELAY_MS);
  }
}

type ExportCsvButtonProps = {
  events: readonly PerpsEvent[];
  queryRange: QueryRange;
};

export function ExportCsvButton({ events, queryRange }: ExportCsvButtonProps) {
  const { account } = useAccount();
  const publicClient = usePublicClient();
  const [isExporting, setIsExporting] = useState(false);
  const [exportedCount, setExportedCount] = useState(0);

  const headers = useMemo(
    () => ({
      pair: m["dex.protrade.tradeHistory.pair"](),
      type: m["dex.protrade.history.type"](),
      direction: m["dex.protrade.tradeHistory.direction"](),
      size: "Size",
      tradeValue: m["dex.protrade.tradeHistory.tradeValue"](),
      price: m["dex.protrade.history.price"](),
      pnl: m["dex.protrade.tradeHistory.pnl"](),
      funding: m["dex.protrade.tradeHistory.funding"](),
      fees: m["dex.protrade.tradeHistory.fees"](),
      makerTaker: m["dex.protrade.tradeHistory.makerTaker"](),
      time: m["dex.protrade.tradeHistory.time"](),
    }),
    [],
  );

  const handleExport = useCallback(async () => {
    if (!account || events.length === 0) return;
    setIsExporting(true);
    setExportedCount(0);

    try {
      const allEvents = await fetchAllPerpsTradeHistory({
        address: account.address,
        client: publicClient,
        queryRange,
        onProgress: setExportedCount,
      });

      if (allEvents.length === 0) return;

      const csv = buildPerpsTradeHistoryCsv(allEvents, headers);
      downloadCsv(tradeHistoryCsvFilename(), csv);
    } catch (error) {
      console.error("Failed to export trade history CSV:", error);
    } finally {
      setIsExporting(false);
      setExportedCount(0);
    }
  }, [account, events.length, headers, publicClient, queryRange]);

  return (
    <div className="inline-flex items-center gap-2">
      <Button
        type="button"
        variant="link"
        size="xs"
        onClick={handleExport}
        isDisabled={!account || events.length === 0 || isExporting}
      >
        {m["dex.protrade.tradeHistory.exportCsv"]()}
      </Button>
      {isExporting ? (
        <output
          aria-label={`Exporting CSV, ${exportedCount} trades`}
          className="inline-flex items-center gap-1 diatype-xs-regular text-primitives-red-light-400 whitespace-nowrap"
        >
          <Spinner size="xs" color="pink" />
          <span>{exportedCount} trades</span>
        </output>
      ) : null}
    </div>
  );
}
