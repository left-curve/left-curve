import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";

export type MarginMode = "cross" | "isolated";

export type PerpsTradeSettingsState = {
  /** User-selected leverage per pairId. Falls back to maxLeverage when unset. */
  leverageByPair: Record<string, number>;
  /** User-selected margin mode per pairId. Defaults to "cross". */
  marginModeByPair: Record<string, MarginMode>;
  /**
   * Persist a leverage value for a pair, clamped to `[1, maxLeverage]` and
   * rounded to an integer. The clamp is enforced at the store boundary so
   * persisted state can never drift outside the pair's allowed range.
   */
  setLeverage: (pairId: string, leverage: number, maxLeverage: number) => void;
  setMarginMode: (pairId: string, mode: MarginMode) => void;
};

/**
 * Persisted store for per-pair perps trade settings (leverage, margin mode).
 *
 * Leverage is keyed by `pairId` (e.g. "perp/btcusd"). When no value is
 * stored for a pair, callers should fall back to the pair's max leverage.
 */
export const perpsTradeSettingsStore = create<PerpsTradeSettingsState>()(
  persist(
    (set) => ({
      leverageByPair: {},
      marginModeByPair: {},
      setLeverage: (pairId, leverage, maxLeverage) => {
        const upperBound = Math.max(1, Math.floor(maxLeverage));
        const clamped = Math.min(Math.max(Math.round(leverage), 1), upperBound);
        set((state) => ({
          leverageByPair: { ...state.leverageByPair, [pairId]: clamped },
        }));
      },
      setMarginMode: (pairId, mode) =>
        set((state) => ({
          marginModeByPair: { ...state.marginModeByPair, [pairId]: mode },
        })),
    }),
    {
      name: "dango.perpsTradeSettings",
      storage: createJSONStorage(() => localStorage),
    },
  ),
);
