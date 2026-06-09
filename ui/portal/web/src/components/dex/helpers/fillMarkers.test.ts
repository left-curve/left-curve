import { describe, expect, it } from "vitest";

import { buildFillMarker, getFillMarkerBarTime } from "./fillMarkers";

import type { OrderFilledData, PerpsEvent } from "@left-curve/types";

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
  it("buckets fill time to the visible chart resolution", () => {
    const fillTimeMs = Date.parse("2026-06-09T00:07:12.000Z");

    expect(getFillMarkerBarTime(fillTimeMs, "5")).toBe(
      Date.parse("2026-06-09T00:05:00.000Z") / 1000,
    );
  });

  it("buckets weekly fills to the indexer's Sunday UTC week start", () => {
    const fillTimeMs = Date.parse("2026-06-09T00:07:12.000Z");

    expect(getFillMarkerBarTime(fillTimeMs, "1W")).toBe(
      Date.parse("2026-06-07T00:00:00.000Z") / 1000,
    );
  });

  it("builds a buy marker from an order_filled event", () => {
    const marker = buildFillMarker(makeEvent(), {
      resolution: "5",
    });

    expect(marker).toMatchObject({
      id: "0x1234567890abcdef1234567890abcdef12345678:7:42:17",
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

  it("skips non-fill events and malformed fill values", () => {
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

    expect(
      buildFillMarker(
        makeEvent({
          data: {
            ...baseFillData,
            fill_price: "not-a-price",
          },
        }),
        {
          resolution: "5",
        },
      ),
    ).toBeNull();
  });
});
