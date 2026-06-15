import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { perpsTradeSettingsStore } from "../../../store/src/stores/perpsTradeSettingsStore";

const initialState = perpsTradeSettingsStore.getInitialState();

function resetStore() {
  perpsTradeSettingsStore.setState(
    {
      leverageByPair: {},
      marginModeByPair: {},
      setLeverage: initialState.setLeverage,
      setMarginMode: initialState.setMarginMode,
    },
    true,
  );
}

describe("perps trade settings store", () => {
  beforeEach(() => {
    localStorage.clear();
    resetStore();
  });

  afterEach(() => {
    localStorage.clear();
    resetStore();
  });

  it("rounds and clamps leverage at the store boundary", () => {
    const state = perpsTradeSettingsStore.getState();

    state.setLeverage("perp/btcusd", 12.6, 50);
    state.setLeverage("perp/ethusd", 0.4, 25);
    state.setLeverage("perp/solusd", 99, 20);
    state.setLeverage("perp/xrpusd", 9, 0);

    expect(perpsTradeSettingsStore.getState().leverageByPair).toEqual({
      "perp/btcusd": 13,
      "perp/ethusd": 1,
      "perp/solusd": 20,
      "perp/xrpusd": 1,
    });
  });

  it("updates per-pair margin modes without disturbing leverage settings", () => {
    const state = perpsTradeSettingsStore.getState();

    state.setLeverage("perp/btcusd", 7, 50);
    state.setMarginMode("perp/btcusd", "isolated");
    state.setMarginMode("perp/ethusd", "cross");

    expect(perpsTradeSettingsStore.getState()).toMatchObject({
      leverageByPair: {
        "perp/btcusd": 7,
      },
      marginModeByPair: {
        "perp/btcusd": "isolated",
        "perp/ethusd": "cross",
      },
    });
  });

  it("persists serializable perps settings to local storage", () => {
    const state = perpsTradeSettingsStore.getState();

    state.setLeverage("perp/btcusd", 8, 50);
    state.setMarginMode("perp/btcusd", "isolated");

    expect(JSON.parse(localStorage.getItem("dango.perpsTradeSettings") as string)).toEqual({
      state: {
        leverageByPair: {
          "perp/btcusd": 8,
        },
        marginModeByPair: {
          "perp/btcusd": "isolated",
        },
      },
      version: 0,
    });
  });
});
