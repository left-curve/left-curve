import { create } from "zustand";
import type { PairId } from "@left-curve/dango/types";

export type TradePairState = {
  mode: "spot" | "perps";
  pairId: PairId;
  setPair: (pairId: PairId, mode: "spot" | "perps") => void;
};

export const tradePairStore = create<TradePairState>((set) => ({
  mode: "spot",
  pairId: { baseDenom: "", quoteDenom: "" },
  setPair: (pairId, mode) => set({ pairId, mode }),
}));

/** Derive perps contract pair id. Symbols passed uppercase, lowercased internally.
 *  e.g. ("ETH", "USD") → "perp/ethusd" */
export function toPerpsPairId(baseSymbol: string, quoteSymbol: string): string {
  return `perp/${baseSymbol.toLowerCase()}${quoteSymbol.toLowerCase()}`;
}
