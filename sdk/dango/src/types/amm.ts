import type { Coin } from "@left-curve/types";
import type { FeeRate, PoolId, PoolParams } from "./pool.js";

export type AmmConfig = {
  /**  The amount of fee that must be paid in order to create a pool. */
  readonly poolCreationFee: Coin;
  /** Percentage of the final swap output that is charged as protocol fee,
   * paid to token stakers.
   *
   * Note to be confused with the liquidity fee, which is configured on a
   * per-pool basis, and paid to liquidity providers. */
  readonly protocolFeeRate: FeeRate;
};

export type AmmQueryMsg =
  /** Query the AMM's global configuration. */
  | { config: Record<never, never> }
  /** Query the state of a single pool by ID. */
  | { pool: { poolId: PoolId } }
  /** Query the states of all pools. */
  | { pools: { startAfter?: PoolId; limit?: number } }
  /** Simulate the output of a swap. */
  | { simulate: { input: Coin; route: PoolId[] } };

export type AmmExecuteMsg =
  /** Create a new trading pool with the given parameters. */
  | { createPool: PoolParams }
  /** Perform a swap. */
  | { swap: { route: PoolId[]; minimumOutput?: string } }
  /** Provide liquidity to a trading pool. */
  | { provideLiquidity: { poolId: PoolId; minimumOutput?: string } }
  /** Withdraw liquidity from a trading pool. */
  | { withdrawLiquidity: { poolId: PoolId } };

export type SwapOutcome = {
  /** The amount of coin to be returned to the trader. */
  output: Coin;
  /** The amount of fee paid to the protocol's token stakers. */
  protocolFee: Coin;
  /** The amount of fee paid to liquidity providers. */
  liquidityFees: Coin[];
};
