import { QueryClient } from "@tanstack/react-query";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

import { buildFillMarker, fetchFillMarkers } from "./fillMarkers";

import type { PublicClient } from "@left-curve/sdk";
import type { OrderFilledData, PerpsEvent } from "@left-curve/types";
import type { ResolutionString } from "@left-curve/tradingview";

const baseFillData: OrderFilledData = {
  order_id: "42",
  pair_id: "perp/btcusd",
  user: "0xuser",
  fill_price: "65000.000000",
  fill_size: "0.100000",
  closing_size: "0.000000",
  opening_size: "0.100000",
  realized_pnl: "0.000000",
  realized_funding: "0.000000",
  fee: "6.500000",
  fill_id: "17",
  is_maker: false,
};

function makeEvent(overrides: Partial<PerpsEvent> = {}): PerpsEvent {
  return {
    idx: 7,
    blockHeight: 123,
    txHash: "0x1234567890abcdef1234567890abcdef12345678",
    eventType: "order_filled",
    userAddr: "0xuser",
    pairId: "perp/btcusd",
    data: baseFillData,
    createdAt: "2026-06-09T00:07:12.000Z",
    ...overrides,
  };
}

describe("fill markers", () => {
  beforeAll(() => {
    vi.stubGlobal("localStorage", {
      getItem: vi.fn(() => null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    });
  });

  afterAll(() => {
    vi.unstubAllGlobals();
  });

  it("buckets fill time to the visible chart resolution", () => {
    const marker = buildFillMarker(makeEvent(), {
      resolution: "5",
    });

    expect(marker?.time).toBe(Date.parse("2026-06-09T00:05:00.000Z") / 1000);
  });

  it("buckets weekly fills to the indexer's Sunday UTC week start", () => {
    const marker = buildFillMarker(makeEvent(), {
      resolution: "1W",
    });

    expect(marker?.time).toBe(Date.parse("2026-06-07T00:00:00.000Z") / 1000);
  });

  it("builds a buy marker from an order_filled event", () => {
    const marker = buildFillMarker(makeEvent(), {
      resolution: "5",
    });

    expect(marker).toMatchObject({
      id: "0x1234567890abcdef1234567890abcdef12345678:7",
      label: "B",
      labelFontColor: "#FFFCF6",
      minSize: 16,
      time: Date.parse("2026-06-09T00:05:00.000Z") / 1000,
      color: {
        border: "#27AE60",
        background: "#27AE60",
      },
    });
    expect(marker?.text).toContain("Buy 0.1 BTC at $65000");
    expect(marker?.text).toContain("Taker");
  });

  it("builds a sell marker from a negative fill size", () => {
    const marker = buildFillMarker(
      makeEvent({
        data: {
          ...baseFillData,
          fill_size: "-0.050000",
          is_maker: true,
        },
      }),
      {
        resolution: "5",
      },
    );

    expect(marker).toMatchObject({
      label: "S",
      color: {
        border: "#EB5757",
        background: "#EB5757",
      },
    });
    expect(marker?.text).toContain("Sell 0.05 BTC at $65000");
    expect(marker?.text).toContain("Maker");
  });

  it("skips non-fill events and zero-size fills", () => {
    expect(
      buildFillMarker(makeEvent({ eventType: "liquidated" }), {
        resolution: "5",
      }),
    ).toBeNull();

    expect(
      buildFillMarker(
        makeEvent({
          data: {
            ...baseFillData,
            fill_size: "0",
          },
        }),
        {
          resolution: "5",
        },
      ),
    ).toBeNull();
  });

  it("pads marker event queries through the current daily bucket", async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const queryPerpsEvents = vi.fn().mockResolvedValue({ nodes: [] });

    await fetchFillMarkers({
      client: { queryPerpsEvents } as unknown as PublicClient,
      queryClient,
      accountAddress: "0xuser",
      pairId: "perp/btcusd",
      resolution: "1D" as ResolutionString,
      from: Date.parse("2026-06-09T00:00:00.000Z") / 1000,
      to: Date.parse("2026-06-09T00:00:00.000Z") / 1000,
    });

    expect(queryPerpsEvents).toHaveBeenCalledWith(
      expect.objectContaining({
        laterThan: "2026-06-09T00:00:00.000Z",
        earlierThan: "2026-06-10T00:00:00.000Z",
      }),
    );
  });
});
