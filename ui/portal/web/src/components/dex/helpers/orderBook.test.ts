import { describe, expect, it } from "vitest";

import { getTopOfBookMidPrice, getTopOfBookPrices } from "./orderBook";

const depth = {
  asks: {
    "101": { notional: "101", size: "1" },
    "103": { notional: "103", size: "1" },
  },
  bids: {
    "97": { notional: "97", size: "1" },
    "99": { notional: "99", size: "1" },
  },
};

describe("order book helpers", () => {
  it("selects the best bid and ask from unsorted depth", () => {
    expect(getTopOfBookPrices(depth)).toEqual({
      bestAsk: "101",
      bestBid: "99",
    });
  });

  it("computes the raw top-of-book midpoint", () => {
    expect(getTopOfBookMidPrice(depth)).toBe("100");
  });

  it("snaps midpoint down or up to tick size", () => {
    const oddSpreadDepth = {
      asks: {
        "100": { notional: "100", size: "1" },
      },
      bids: {
        "99": { notional: "99", size: "1" },
      },
    };

    expect(
      getTopOfBookMidPrice(oddSpreadDepth, { snapDirection: "down", tickSize: "1" }),
    ).toBe("99");
    expect(getTopOfBookMidPrice(oddSpreadDepth, { snapDirection: "up", tickSize: "1" })).toBe(
      "100",
    );
  });

  it("does not return a non-positive midpoint", () => {
    expect(
      getTopOfBookMidPrice({
        asks: {
          "0": { notional: "0", size: "1" },
        },
        bids: {
          "0": { notional: "0", size: "1" },
        },
      }),
    ).toBeNull();
  });
});
