import type { Coin } from "@left-curve/types";

export type PoolId = number;

export type PoolTypes = (typeof PoolType)[keyof typeof PoolType];

export const PoolType = {
  Xyk: "xyk",
  Concentrated: "concentrated",
} as const;

export type PoolParams =
  | { xyk: XykParams }
  // break
  | { concentrated: ConcentratedParams };

export type Pool<T = PoolInfo> = Record<PoolTypes, T>;

export type PoolInfo = XykPool | ConcentratedPool;

export type XykPool = {
  /** The pool's parameters. */
  readonly params: XykParams;
  /**  The amount of liquidity provided to this pool. */
  readonly liquidity: [Coin, Coin];
  /** The total amount of liquidity shares outstanding. */
  readonly shares: string;
};

export type XykParams = {
  /** Percentage of swap output that is charged as liquidity fee, paid to
   * liquidity providers of the pool. */
  readonly liquidityFeeRate: FeeRate;
};

export type ConcentratedPool = {
  /** The pool's parameters. */
  readonly params: ConcentratedParams;
  /**  The amount of liquidity provided to this pool. */
  readonly liquidity: [Coin, Coin];
  /** The total amount of liquidity shares outstanding. */
  readonly shares: string;
};

export type ConcentratedParams = null;

export type FeeRate = string;
