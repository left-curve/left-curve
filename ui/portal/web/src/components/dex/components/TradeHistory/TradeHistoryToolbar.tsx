import { useCallback, useMemo, useState } from "react";
import { Button, DateRangePicker, Select, twMerge, useMediaQuery } from "@left-curve/applets-kit";
import { useAccount, usePublicClient } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  PRESETS,
  type TradeHistoryPreset,
  useTradeHistoryFilter,
} from "./tradeHistoryFilterContext";
import { buildPerpsTradeHistoryCsv, downloadCsv, tradeHistoryCsvFilename } from "./exportCsv";

const EXPORT_CSV_FETCH_LIMIT = 1000;

export const TradeHistoryToolbar: React.FC = () => {
  const { filter, setPreset, setCustomRange } = useTradeHistoryFilter();
  const { account } = useAccount();
  const publicClient = usePublicClient();
  const { isMd } = useMediaQuery();
  const [isExporting, setIsExporting] = useState(false);

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
    if (!account || isExporting) return;
    setIsExporting(true);
    try {
      const earlierThan = filter.to.toISOString();
      const laterThan = filter.from.toISOString();
      const result = await publicClient.queryPerpsEvents({
        userAddr: account.address,
        sortBy: "BLOCK_HEIGHT_DESC",
        first: EXPORT_CSV_FETCH_LIMIT,
        earlierThan,
        laterThan,
      });
      const csv = buildPerpsTradeHistoryCsv(result.nodes, headers);
      downloadCsv(tradeHistoryCsvFilename("perps"), csv);
    } finally {
      setIsExporting(false);
    }
  }, [account, headers, isExporting, publicClient, filter.from, filter.to]);

  const datePicker = (
    <DateRangePicker
      value={{ from: filter.from, to: filter.to }}
      onChange={(value) => {
        if (value.from && value.to) setCustomRange(value.from, value.to);
      }}
      disabled={(date) => date > new Date()}
      triggerClassName="shrink-0"
    />
  );

  const exportButton = (
    <Button
      type="button"
      variant="link"
      size="xs"
      onClick={handleExport}
      isDisabled={!account}
      isLoading={isExporting}
    >
      {m["dex.protrade.tradeHistory.exportCsv"]()}
    </Button>
  );

  const presetButtons = PRESETS.map((preset) => (
    <Button
      key={preset.id}
      type="button"
      variant="link"
      size="xs"
      onClick={() => setPreset(preset.id)}
      className={twMerge(filter.preset === preset.id && "bg-surface-primary-blue")}
    >
      {preset.label}
    </Button>
  ));

  if (isMd) {
    return (
      <div className="flex items-center justify-between gap-4 py-2 px-1">
        <div className="flex items-center gap-2 flex-wrap">
          {presetButtons}
          <span aria-hidden className="shrink-0 w-px h-4 bg-outline-secondary-gray mx-1" />
          {datePicker}
        </div>
        {exportButton}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3 py-2 px-1">
      <div className="flex items-center justify-between gap-3">
        <Select
          value={filter.preset ?? "custom"}
          onChange={(v) => {
            if (v !== "custom") setPreset(v as TradeHistoryPreset);
          }}
          classNames={{
            trigger: "py-1.5 px-3 exposure-xs-italic text-ink-secondary-blue",
          }}
        >
          {PRESETS.map((preset) => (
            <Select.Item key={preset.id} value={preset.id}>
              {preset.label}
            </Select.Item>
          ))}
          {filter.preset === null && (
            <Select.Item value="custom">{m["dex.protrade.tradeHistory.customDate"]()}</Select.Item>
          )}
        </Select>
        {datePicker}
      </div>
      <div className="flex justify-end">{exportButton}</div>
    </div>
  );
};
