import type { Denom, Option } from "./index.js";

export type PairId = {
  baseDenom: string;
  quoteDenom: string;
};

export type PairSymbols = {
  baseSymbol: string;
  quoteSymbol: string;
};

export type OrderId = string;

export const Direction = {
  /** Give away the quote asset, get the base asset; a.k.a. a BUY order. */
  Buy: "bid",
  /** Give away the base asset, get the quote asset; a.k.a. a SELL order. */
  Sell: "ask",
} as const;

export type Directions = (typeof Direction)[keyof typeof Direction];

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
  /** Price buckets for the liquidity depth chart. */
  bucketSizes: string[];
  /** Minimum order size, defined _in the base asset_. */
  minOrderSizeBase: string;
  /** Minimum order size, defined _in the quote asset_. */
  minOrderSizeQuote: string;
};

export type PairUpdate = {
  baseDenom: Denom;
  quoteDenom: Denom;
  params: PairParams;
};

export const TimeInForceOption = {
  GoodTilCanceled: "GTC",
  ImmediateOrCancel: "IOC",
};

export type TimeInForceOptions = (typeof TimeInForceOption)[keyof typeof TimeInForceOption];

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

export const OrderType = {
  Limit: "limit",
  Market: "market",
} as const;

export type OrderTypes = (typeof OrderType)[keyof typeof OrderType];

export type CandleIntervals = (typeof CandleInterval)[keyof typeof CandleInterval];

export type PerpsCandle = {
  pairId: string;
  interval: CandleIntervals;
  minBlockHeight: number;
  maxBlockHeight: number;
  open: string;
  high: string;
  low: string;
  close: string;
  volume: string;
  volumeUsd: string;
  timeStart: string;
  timeStartUnix: number;
  timeEnd: string;
  timeEndUnix: number;
};

export type PerpsPairStats = {
  pairId: string;
  currentPrice: Option<string>;
  price24HAgo: Option<string>;
  volume24H: string;
  priceChange24H: Option<string>;
};
