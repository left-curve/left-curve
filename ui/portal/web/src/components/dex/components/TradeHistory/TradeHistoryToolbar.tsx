import { Button, DateRangePicker, Select, twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  PRESETS,
  type TradeHistoryPreset,
  useTradeHistoryFilter,
} from "./tradeHistoryFilterContext";

const PRESET_LABELS: Record<TradeHistoryPreset, () => string> = {
  "1d": m["dex.protrade.tradeHistory.preset.1d"],
  "1w": m["dex.protrade.tradeHistory.preset.1w"],
  "1m": m["dex.protrade.tradeHistory.preset.1m"],
  "3m": m["dex.protrade.tradeHistory.preset.3m"],
};

type TradeHistoryToolbarProps = {
  layout: "desktop" | "mobile";
};

export function TradeHistoryToolbar({ layout }: TradeHistoryToolbarProps) {
  const { filter, setPreset, setCustomRange } = useTradeHistoryFilter();

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

  if (layout === "desktop") {
    return (
      <div className="flex items-center gap-2 flex-wrap">
        {PRESETS.map((preset) => (
          <Button
            key={preset.id}
            type="button"
            variant="link"
            size="xs"
            onClick={() => setPreset(preset.id)}
            className={twMerge(filter.preset === preset.id && "bg-surface-primary-blue")}
          >
            {PRESET_LABELS[preset.id]()}
          </Button>
        ))}
        <span aria-hidden className="shrink-0 w-px h-4 bg-outline-secondary-gray mx-1" />
        {datePicker}
      </div>
    );
  }

  return (
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
            {PRESET_LABELS[preset.id]()}
          </Select.Item>
        ))}
        {filter.preset === null && (
          <Select.Item value="custom">{m["dex.protrade.tradeHistory.customDate"]()}</Select.Item>
        )}
      </Select>
      {datePicker}
    </div>
  );
}
