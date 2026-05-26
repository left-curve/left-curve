import { Button } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { PerpsEvent } from "@left-curve/types";
import { useCallback, useMemo } from "react";

import { buildPerpsTradeHistoryCsv, downloadCsv, tradeHistoryCsvFilename } from "./exportCsv";

type ExportCsvButtonProps = {
  events: readonly PerpsEvent[];
};

export function ExportCsvButton({ events }: ExportCsvButtonProps) {
  const { account } = useAccount();

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

  const handleExport = useCallback(() => {
    if (!account || events.length === 0) return;
    const csv = buildPerpsTradeHistoryCsv(events, headers);
    downloadCsv(tradeHistoryCsvFilename(), csv);
  }, [account, events, headers]);

  return (
    <Button
      type="button"
      variant="link"
      size="xs"
      onClick={handleExport}
      isDisabled={!account || events.length === 0}
    >
      {m["dex.protrade.tradeHistory.exportCsv"]()}
    </Button>
  );
}
