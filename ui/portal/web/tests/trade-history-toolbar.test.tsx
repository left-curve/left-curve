import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { TradeHistoryToolbar } from "../src/components/dex/components/TradeHistory/TradeHistoryToolbar";

import type {
  TradeHistoryFilter,
  TradeHistoryPreset,
} from "../src/components/dex/components/TradeHistory/useTradeHistoryFilter";

class TestResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

vi.stubGlobal("ResizeObserver", TestResizeObserver);

const filter: TradeHistoryFilter = {
  from: new Date("2026-05-09T00:00:00.000Z"),
  preset: "1m",
  to: new Date("2026-06-09T00:00:00.000Z"),
};

describe("trade history toolbar", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-06-10T12:00:00.000Z"));
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("routes desktop preset buttons and custom date ranges to the filter callbacks", () => {
    const onPresetChange = vi.fn();
    const onCustomRangeChange = vi.fn();

    render(
      <TradeHistoryToolbar
        layout="desktop"
        filter={filter}
        onCustomRangeChange={onCustomRangeChange}
        onPresetChange={onPresetChange}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: m["dex.protrade.tradeHistory.preset.1w"]() }),
    );

    expect(onPresetChange).toHaveBeenCalledWith("1w");

    fireEvent.click(screen.getByRole("button", { name: /May 9th, 2026.*Jun 9th, 2026/ }));
    fireEvent.click(screen.getByRole("button", { name: /Monday, June 1st, 2026/ }));
    fireEvent.click(screen.getByRole("button", { name: /Monday, June 8th, 2026/ }));

    expect(onCustomRangeChange).toHaveBeenCalledWith(new Date(2026, 5, 1), new Date(2026, 5, 8));
  });

  it("routes mobile preset selections and keeps a custom-range option visible", () => {
    const onPresetChange = vi.fn<(preset: TradeHistoryPreset) => void>();
    const onCustomRangeChange = vi.fn();

    render(
      <TradeHistoryToolbar
        layout="mobile"
        filter={{ ...filter, preset: null }}
        onCustomRangeChange={onCustomRangeChange}
        onPresetChange={onPresetChange}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: m["dex.protrade.tradeHistory.customDate"]() }),
    );

    expect(
      within(screen.getByRole("list")).getByText(m["dex.protrade.tradeHistory.customDate"]()),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText(m["dex.protrade.tradeHistory.preset.3m"]()));

    expect(onPresetChange).toHaveBeenCalledWith("3m");
  });
});
