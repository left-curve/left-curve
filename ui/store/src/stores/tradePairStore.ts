import { create } from "zustand";
import { CoinStore } from "./coinStore.js";

import type { PairId } from "@left-curve/dango/types";

export type TradePairState = {
  mode: "spot" | "perps";
  pairId: PairId;
  setPair: (pairId: PairId, mode: "spot" | "perps") => void;
  getPerpsPairId: (pairId?: PairId) => string;
};

export const TradePairStore = create<TradePairState>((set, get) => ({
  mode: "spot",
  pairId: { baseDenom: "", quoteDenom: "" },
  setPair: (pairId, mode) => {
    const current = get();
    if (
      current.pairId.baseDenom === pairId.baseDenom &&
      current.pairId.quoteDenom === pairId.quoteDenom &&
      current.mode === mode
    ) {
      return;
    }
    set({ pairId, mode });
  },
  getPerpsPairId: (_pairId_) => {
    const pairId = _pairId_ ?? get().pairId;
    const coinStore = CoinStore.getState();
    if (!pairId) throw new Error("[TradePairStore] pairId is not set");
    const base = coinStore.byDenom[pairId.baseDenom];
    if (!base) return "";
    return `perp/${base.symbol.toLowerCase()}usd`;
  },
}));
