import { create } from "zustand";

export type TradeInfoState = {
  operation: "limit" | "market";
  action: "buy" | "sell";
  setOperation: (op: "limit" | "market") => void;
  setAction: (action: "buy" | "sell") => void;
};

export const tradeInfoStore = create<TradeInfoState>((set) => ({
  operation: "market",
  action: "buy",
  setOperation: (operation) => set({ operation }),
  setAction: (action) => set({ action }),
}));
