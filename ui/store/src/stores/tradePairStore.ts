import { create } from "zustand";
import { CoinStore } from "./coinStore.js";

import type { PairId } from "@left-curve/types";

export type TradePairState = {
  pairId: PairId;
  setPair: (pairId: PairId) => void;
  getPerpsPairId: (pairId?: PairId) => string;
};

export const TradePairStore = create<TradePairState>((set, get) => ({
  pairId: { baseDenom: "", quoteDenom: "" },
  setPair: (pairId) => {
    const current = get();
    if (
      current.pairId.baseDenom === pairId.baseDenom &&
      current.pairId.quoteDenom === pairId.quoteDenom
    ) {
      return;
    }
    set({ pairId });
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
