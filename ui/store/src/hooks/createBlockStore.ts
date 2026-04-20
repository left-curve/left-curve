import { create } from "zustand";

type BlockGuardConfig<T> = {
  /** Initial data state (excluding block-height guard fields) */
  initialState: T;
  /**
   * Optional callback invoked before each update.
   * Receives the previous state and the incoming data, returns extra fields to merge.
   * Useful for tracking `previousPrice` from `currentPrice`.
   */
  beforeUpdate?: (prev: T, next: Partial<T>) => Partial<T>;
};

export type BlockGuardedState<T> = T & {
  lastUpdatedBlockHeight: number;
  setState: (params: Partial<T> & { blockHeight: number }) => void;
};

/**
 * Factory for creating zustand stores with block-height deduplication.
 *
 * All subscription stores in this app follow the same pattern:
 * - Hold domain data (orderBook, liquidityDepth, userState, etc.)
 * - Track `lastUpdatedBlockHeight` to skip stale updates
 * - Provide a `setState` that only applies when `blockHeight > lastUpdatedBlockHeight`
 *
 * This factory eliminates that repetition.
 *
 * @example
 * ```ts
 * const orderBookStore = createBlockStore({
 *   initialState: { orderBook: null, currentPrice: "0", previousPrice: "0" },
 *   beforeUpdate: (prev) => ({ previousPrice: prev.currentPrice }),
 * });
 * ```
 */
export function createBlockStore<T extends Record<string, unknown>>(config: BlockGuardConfig<T>) {
  const { initialState, beforeUpdate } = config;

  return create<BlockGuardedState<T>>((set, get) => ({
    ...initialState,
    lastUpdatedBlockHeight: 0,
    setState: (params) => {
      const { blockHeight, ...data } = params;
      const current = get();
      // blockHeight === 0 means HTTP polling mode (no block info) — always accept
      if (blockHeight > 0 && blockHeight <= current.lastUpdatedBlockHeight) return;

      const extra = beforeUpdate
        ? beforeUpdate(current as unknown as T, data as unknown as Partial<T>)
        : {};

      set({
        ...(data as object),
        ...(extra as object),
        lastUpdatedBlockHeight: blockHeight,
      } as Partial<BlockGuardedState<T>>);
    },
  }));
}
