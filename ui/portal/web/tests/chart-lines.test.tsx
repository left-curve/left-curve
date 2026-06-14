import { waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  buildPerpsOrderLines,
  buildPositionLines,
  drawLines,
} from "../src/components/dex/helpers/chartLines";

import type { PerpsOrdersByUserResponse, PerpsPositionExtended } from "@left-curve/types";

const colors = {
  buy: "#27AE60",
  sell: "#EB5757",
};

describe("DEX chart lines", () => {
  it("maps a long backend position into entry, liquidation, take-profit, and stop-loss lines", () => {
    const position = {
      conditionalOrderAbove: {
        triggerPrice: "33000",
      },
      conditionalOrderBelow: {
        triggerPrice: "28000",
      },
      entryPrice: "30000",
      liquidationPrice: "25000",
      size: "0.5",
    } as PerpsPositionExtended;

    expect(buildPositionLines(position)).toEqual([
      {
        color: colors.buy,
        linestyle: 0,
        price: 30000,
      },
      {
        color: colors.sell,
        linestyle: 1,
        price: 25000,
      },
      {
        color: colors.buy,
        linestyle: 2,
        price: 33000,
      },
      {
        color: colors.sell,
        linestyle: 2,
        price: 28000,
      },
    ]);
  });

  it("inverts TP and SL trigger direction for short backend positions", () => {
    const position = {
      conditionalOrderAbove: {
        triggerPrice: "2200",
      },
      conditionalOrderBelow: {
        triggerPrice: "1800",
      },
      entryPrice: "2000",
      liquidationPrice: "2500",
      size: "-1",
    } as PerpsPositionExtended;

    expect(buildPositionLines(position)).toEqual([
      {
        color: colors.sell,
        linestyle: 0,
        price: 2000,
      },
      {
        color: colors.sell,
        linestyle: 1,
        price: 2500,
      },
      {
        color: colors.buy,
        linestyle: 2,
        price: 1800,
      },
      {
        color: colors.sell,
        linestyle: 2,
        price: 2200,
      },
    ]);
  });

  it("filters backend open orders to the active pair and colors them by signed size", () => {
    const orders = {
      "ask-eth": {
        limitPrice: "2100",
        pairId: "perp/ethusd",
        size: "-2",
      },
      "bid-btc": {
        limitPrice: "30000",
        pairId: "perp/btcusd",
        size: "0.25",
      },
      "other-pair": {
        limitPrice: "40000",
        pairId: "perp/ethusd",
        size: "1",
      },
    } as PerpsOrdersByUserResponse;

    expect(buildPerpsOrderLines(orders, "perp/ethusd")).toEqual([
      {
        color: colors.sell,
        linestyle: 2,
        price: 2100,
      },
      {
        color: colors.buy,
        linestyle: 2,
        price: 40000,
      },
    ]);
  });

  it("replaces unsaved TradingView shapes before drawing current backend lines", async () => {
    const savedShape = {
      id: "saved",
    };
    const unsavedShape = {
      id: "generated",
    };
    const chart = {
      createShape: vi.fn().mockResolvedValue(undefined),
      getAllShapes: vi.fn(() => [savedShape, unsavedShape]),
      getShapeById: vi.fn((id: string) => ({
        isSavingEnabled: () => id === savedShape.id,
      })),
      removeEntity: vi.fn(),
    };

    drawLines(chart, [
      {
        color: colors.buy,
        linestyle: 0,
        price: 30000,
      },
      {
        color: colors.sell,
        linestyle: 2,
        price: 28000,
      },
    ]);

    await waitFor(() => expect(chart.createShape).toHaveBeenCalledTimes(2));

    expect(chart.removeEntity).toHaveBeenCalledWith(unsavedShape.id);
    expect(chart.removeEntity).not.toHaveBeenCalledWith(savedShape.id);
    expect(chart.createShape).toHaveBeenCalledWith(
      {
        price: 30000,
      },
      expect.objectContaining({
        disableSave: true,
        disableSelection: true,
        lock: true,
        overrides: expect.objectContaining({
          linecolor: colors.buy,
          linestyle: 0,
          showPrice: true,
        }),
        shape: "horizontal_line",
      }),
    );
  });
});
