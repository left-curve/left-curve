import type {
  Address,
  Coin,
  Denom,
  ExtractFromUnion,
  KeyOfUnion,
  Option,
  Timestamp,
} from "@left-curve/sdk/types";
import type { Username } from "./account.js";

export type SwapRoute = PairId[];

export type DexQueryMsg =
  /** Returns the trading volume of a username since the specified timestamp. */
  | {
      volumeByUser: {
        /** The username to query trading volume for. */
        user: Username;
        /** The start timestamp to query trading volume for. If not provided,
         * username's total trading volume will be returned. */
        since: Option<Timestamp>;
      };
    }
  /** Returns the trading volume of a user address since the specified timestamp. */
  | {
      volume: {
        /** The user's address to query trading volume for. */
        user: Address;
        /** The start timestamp to query trading volume for. If not provided,
         * user's total trading volume will be returned. */
        since: Option<Timestamp>;
      };
    }
  /** Simulate a swap with exact output. */
  | {
      simulateSwapExactAmountOut: {
        /** The route of the swap. */
        route: SwapRoute;
        /** The output amount of the swap. */
        output: Coin;
      };
    }
  /** Simulate a swap with exact input. */
  | {
      simulateSwapExactAmountIn: {
        /** The route of the swap. */
        route: SwapRoute;
        /** The input amount of the swap. */
        input: Coin;
      };
    }
  /** Query the parameters of a single trading pair. */
  | {
      pair: {
        /** The base denomination of the trading pair. */
        baseDenom: Denom;
        /** The quote denomination of the trading pair. */
        quoteDenom: Denom;
      };
    }
  /** Enumerate all trading pairs and their parameters. */
  | {
      pairs: {
        /** The ID of the pair to start after. */
        startAfter: Option<PairId>;
        /** The maximum number of pairs to return. */
        limit: Option<number>;
      };
    }
  /** Query the passive liquidity pool reserve of a single trading pair. */
  | {
      reserve: {
        /** The base denomination of the trading pair. */
        baseDenom: Denom;
        /** The quote denomination of the trading pair. */
        quoteDenom: Denom;
      };
    }
  /** Enumerate all passive liquidity pool reserves. */
  | {
      reserves: {
        /** The ID of the reserve to start after. */
        startAfter: Option<PairId>;
        /** The maximum number of reserves to return. */
        limit: Option<number>;
      };
    }
  /** Simulate a liquidity withdrawal. */
  | {
      simulateWithdrawLiquidity: {
        /** The base denomination of the trading pair. */
        baseDenom: Denom;
        /** The quote denomination of the trading pair. */
        quoteDenom: Denom;
        /** The amount of liquidity to withdraw, in the liquidity token denom. */
        lpBurnAmount: string;
      };
    }
  /** Query a single active order by ID. */
  | {
      order: {
        /** The ID of the order to query. */
        orderId: OrderId;
      };
    }
  /** Enumerate active orders across all pairs and from users. */
  | {
      orders: {
        /** The ID of the order to start after. */
        startAfter: Option<OrderId>;
        /** The maximum number of orders to return. */
        limit: Option<number>;
      };
    }
  /** Enumerate active orders in a single pair from all users. */
  | {
      ordersByPair: {
        /** The base denomination of the trading pair. */
        baseDenom: Denom;
        /** The quote denomination of the trading pair. */
        quoteDenom: Denom;
        /** The ID of the order to start after. */
        startAfter: Option<OrderId>;
        /** The maximum number of orders to return. */
        limit: Option<number>;
      };
    }
  /** Enumerate active orders from a single user across all pairs. */
  | {
      ordersByUser: {
        /** The user address to query. */
        user: Address;
        /** The ID of the order to start after. */
        startAfter?: Option<OrderId>;
        /** The maximum number of orders to return. */
        limit?: Option<number>;
      };
    };

export type DexExecuteMsg =
  /**
   * Perform an instant swap directly in the passive liqudiity pools, with an
   * exact amount of output asset.
   *
   * User must send exactly one asset, which must be either the base or quote
   * asset of the first pair in the `route`.
   *
   * Slippage control is implied by the input amount. If required input is
   * less than what user sends, the excess is refunded. Otherwise, if required
   * input more than what user sends, the swap fails.
   */
  | {
      swapExactAmountOut: { route: SwapRoute; output: Coin };
    }
  /**
   * Perform an instant swap directly in the passive liquidity pools, with an
   * exact amount of input asset.
   * User must send exactly one asset, which must be either the base or quote
   * asset of the first pair in the `route`.
   *
   * User may specify a minimum amount of output, for slippage control.
   */
  | {
      swapExactAmountIn: { route: SwapRoute; minimumOutput: Option<string> };
    }
  /**
   * Create or cancel multiple limit orders in one batch. */
  | {
      batchUpdateOrders: {
        createsMarket: CreateMarketOrderRequest[];
        createsLimit: CreateLimitOrderRequest[];
        cancels: Option<CancelOrderRequest>;
      };
    }
  /**
   * Withdraw passive liquidity from a pair. Withdrawal is always performed at
   * the pool ratio.
   */
  | {
      withdrawLiquidity: {
        baseDenom: string;
        quoteDenom: string;
      };
    }
  /**
   * Provide passive liquidity to a pair. Unbalanced liquidity provision is
   * equivalent to a swap to reach the pool ratio, followed by a liquidity
   * provision at pool ratio.
   */
  | {
      provideLiquidity: {
        baseDenom: string;
        quoteDenom: string;
      };
    };

export type GetDexExecuteMsg<K extends KeyOfUnion<DexExecuteMsg>> = ExtractFromUnion<
  DexExecuteMsg,
  K
>;

export type GetDexQueryMsg<K extends KeyOfUnion<DexQueryMsg>> = ExtractFromUnion<DexQueryMsg, K>;

export type PairId = {
  baseDenom: string;
  quoteDenom: string;
};

export type PairSymbols = {
  baseSymbol: string;
  quoteSymbol: string;
};

export type OrderId = string;

export type CoinPair = [Coin, Coin];

export type ReservesResponse = {
  pair: PairId;
  reserve: CoinPair;
};

export const Direction = {
  /** Give away the quote asset, get the base asset; a.k.a. a BUY order. */
  Buy: "bid",
  /** Give away the base asset, get the quote asset; a.k.a. a SELL order. */
  Sell: "ask",
} as const;

export type Directions = (typeof Direction)[keyof typeof Direction];

export type OrderResponse = {
  user: Address;
  baseDenom: string;
  quoteDenom: string;
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
  baseDenom: string;
  quoteDenom: string;
  direction: Directions;
  price: string;
  amount: string;
  remaining: string;
};

export const CurveInvariant = {
  XYK: "xyk",
} as const;

export type CurveInvariants = (typeof CurveInvariant)[keyof typeof CurveInvariant];

export type PairParams = {
  /**  Liquidity token denom of the passive liquidity pool */
  lpDenom: Denom;
  /**  Curve invariant for the passive liquidity pool. */
  curveInvariant: CurveInvariants;
  /**  Fee rate for instant swaps in the passive liquidity pool. */
  swapFeeRate: string;
};

export type PairUpdate = {
  baseDenom: Denom;
  quoteDenom: Denom;
  params: PairParams;
};

export type CancelOrderRequest = "all" | { some: OrderId[] };

export type CreateLimitOrderRequest = {
  baseDenom: Denom;
  quoteDenom: Denom;
  direction: Directions;
  /** The amount of _base asset_ to trade.
   *
   * The frontend UI may allow user to choose the amount in terms of the
   * quote asset, and convert it to the base asset amount behind the scene:
   *
   * ```plain
   * base_asset_amount = floor(quote_asset_amount / price)
   * ```
   */
  amount: string;
  /** The limit price measured _in the quote asset_, i.e. how many units of
   * quote asset is equal in value to 1 unit of base asset.
   */
  price: string;
};

export type CreateMarketOrderRequest = {
  baseDenom: Denom;
  quoteDenom: Denom;
  direction: Directions;
  /**
   * For BUY orders, the amount of quote asset; for SELL orders, that of the
   * base asset.
   */
  amount: string;
  /**
   * The maximum slippage percentage.
   *
   * This parameter works as follow:
   *
   * - For a market BUY order, suppose the best (lowest) SELL price in the
   *   resting order book is `p_best`, then the market order's _average
   *   execution price_ can't be worse than:
   *
   *   ```math
   *   p_best * (1 + max_slippage)
   *   ```
   *
   * - For a market SELL order, suppose the best (highest) BUY price in the
   *   resting order book is `p_best`, then the market order's _average
   *   execution price_ can't be worse than:
   *
   *   ```math
   *   p_best * (1 - max_slippage)
   *   ```
   *
   * Market orders are _immediate or cancel_ (IOC), meaning, if there isn't
   * enough liquidity in the resting order book to fully fill the market
   * order under its max slippage, it's filled as much as possible, with the
   * unfilled portion is canceled.
   */
  maxSlippage: string;
};

export const CandleInterval = {
  OneSecond: "ONE_SECOND",
  OneMinute: "ONE_MINUTE",
  FiveMinutes: "FIVE_MINUTES",
  FifteenMinutes: "FIFTEEN_MINUTES",
  OneHour: "ONE_HOUR",
  FourHours: "FOUR_HOURS",
  OneDay: "ONE_DAY",
  OneWeek: "ONE_WEEK",
} as const;

export type CandleIntervals = (typeof CandleInterval)[keyof typeof CandleInterval];

export type Candle = {
  quoteDenom: Denom;
  baseDenom: Denom;
  interval: CandleIntervals;
  blockHeight: number;
  open: string;
  high: string;
  low: string;
  close: string;
  volumeBase: string;
  volumeQuote: string;
  timeStart: string;
  timeStartUnix: number;
  timeEnd: string;
  timeEndUnix: number;
};

export type Trade = {
  addr: Address;
  quoteDenom: Denom;
  baseDenom: Denom;
  direction: Directions;
  blockHeight: number;
  tradeIdx: number;
  createdAt: string;
  filledBase: string;
  filledQuote: string;
  refundBase: string;
  refundQuote: string;
  feeBase: string;
  feeQuote: string;
  clearingPrice: string;
};
