import type { Address, Coin, Timestamp } from "@left-curve/sdk/types";
import type { Username } from "./account.js";
import type { PoolId } from "./pool.js";

export type SwapRoute = [PoolId, PoolId];

export type DexQueryMsg =
  /* Returns the trading volume of a username since the specified timestamp. */
  | {
      volumeByUser: {
        /* The username to query trading volume for. */
        user: Username;
        /* The start timestamp to query trading volume for. If not provided,
         * username's total trading volume will be returned. */
        since?: Timestamp;
      };
    }
  /* Returns the trading volume of a user address since the specified timestamp. */
  | {
      volume: {
        /* The user's address to query trading volume for. */
        user: Address;
        /* The start timestamp to query trading volume for. If not provided,
         * user's total trading volume will be returned. */
        since?: Timestamp;
      };
    }
  /* Simulate a swap with exact output. */
  | {
      simulateSwapExactAmountOut: {
        /* The route of the swap. */
        route: SwapRoute;
        /* The output amount of the swap. */
        output: Coin;
      };
    }
  /* Simulate a swap with exact input. */
  | {
      simulateSwapExactAmountIn: {
        /* The route of the swap. */
        route: SwapRoute;
        /* The input amount of the swap. */
        input: Coin;
      };
    }
  /* Query the parameters of a single trading pair. */
  | {
      pair: {
        /* The base denomination of the trading pair. */
        baseDenom: string;
        /* The quote denomination of the trading pair. */
        quoteDenom: string;
      };
    }
  /* Enumerate all trading pairs and their parameters. */
  | {
      pairs: {
        /* The ID of the pair to start after. */
        startAfter?: PoolId;
        /* The maximum number of pairs to return. */
        limit?: number;
      };
    }
  /* Query the passive liquidity pool reserve of a single trading pair. */
  | {
      reserve: {
        /* The base denomination of the trading pair. */
        baseDenom: string;
        /* The quote denomination of the trading pair. */
        quoteDenom: string;
      };
    }
  /* Enumerate all passive liquidity pool reserves. */
  | {
      reserves: {
        /* The ID of the reserve to start after. */
        startAfter?: PoolId;
        /* The maximum number of reserves to return. */
        limit?: number;
      };
    }
  /* Query a single active order by ID. */
  | {
      order: {
        /* The ID of the order to query. */
        orderId: string;
      };
    }
  /* Enumerate active orders across all pairs and from users. */
  | {
      orders: {
        /* The ID of the order to start after. */
        startAfter?: string;
        /* The maximum number of orders to return. */
        limit?: number;
        /* The ID of the pool to query. */
        poolId: PoolId;
      };
    }
  /* Enumerate active orders in a single pair from all users. */
  | {
      ordersByPair: {
        /* The base denomination of the trading pair. */
        baseDenom: string;
        /* The quote denomination of the trading pair. */
        quoteDenom: string;
        /* The ID of the order to start after. */
        startAfter?: string;
        /* The maximum number of orders to return. */
        limit?: number;
      };
    }
  /* Enumerate active orders from a single user across all pairs. */
  | {
      ordersByUser: {
        /* The user address to query. */
        user: Address;
        /* The ID of the order to start after. */
        startAfter?: string;
        /* The maximum number of orders to return. */
        limit?: number;
      };
    };

export type DexExecuteMsg = {
  /*
  Perform an instant swap directly in the passive liqudiity pools, with an
  exact amount of output asset.

  User must send exactly one asset, which must be either the base or quote
  asset of the first pair in the `route`.

  Slippage control is implied by the input amount. If required input is
  less than what user sends, the excess is refunded. Otherwise, if required
  input more than what user sends, the swap fails.
  */
  swapExactAmountOut: {
    route: SwapRoute;
    output: Coin;
  };
  /*
  Perform an instant swap directly in the passive liqudiity pools, with an
  exact amount of input asset.

  User must send exactly one asset, which must be either the base or quote
  asset of the first pair in the `route`.

  User may specify a minimum amount of output, for slippage control.
  */
  swapExactAmountIn: {
    route: SwapRoute;
    minimum_output: Coin;
  };
  /*
   Withdraw passive liquidity from a pair. Withdrawal is always performed at
    the pool ratio.
  */
  withdrawLiquidity: {
    base_denom: string;
    quote_denom: string;
  };
  /*
   Provide passive liquidity to a pair. Unbalanced liquidity provision is
    equivalent to a swap to reach the pool ratio, followed by a liquidity
    provision at pool ratio.
  */
  provideLiquidity: {
    base_denom: string;
    quote_denom: string;
  };
};

export type PairId = {
  base_denom: string;
  quote_denom: string;
};

export type CoinPair = [Coin, Coin];

export type ReservesResponse = {
  pair: PairId;
  reserve: CoinPair;
};

export const Direction = {
  Bid: 0,
  Ask: 1,
};

export type Directions = (typeof Direction)[keyof typeof Direction];

export type OrderResponse = {
  user: Address;
  base_denom: string;
  quote_denom: string;
  direction: Directions;
  price: string;
  amount: string;
  remaining: string;
};

export type OrdersByPairResponse = {
  user: Address;
  direction: Directions;
  price: string;
  amount: string;
  remaining: string;
};

export type OrdersByUserResponse = {
  base_denom: string;
  quote_denom: string;
  direction: Directions;
  price: string;
  amount: string;
  remaining: string;
};
