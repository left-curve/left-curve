import type { Address, ExtractFromUnion, KeyOfUnion } from "@left-curve/sdk/types";

export type TriggerDirection = "above" | "below";

export type ChildOrder = {
  triggerPrice: string;
  maxSlippage: string;
  size?: string;
};

export type ConditionalOrder = {
  orderId: string;
  size?: string;
  triggerPrice: string;
  maxSlippage: string;
};

export type PerpsPosition = {
  size: string;
  entryPrice: string;
  entryFundingPerUnit: string;
  conditionalOrderAbove?: ConditionalOrder;
  conditionalOrderBelow?: ConditionalOrder;
};

export type PerpsUnlock = {
  endTime: string;
  amountToRelease: string;
};

export type PerpsUserState = {
  margin: string;
  vaultShares: string;
  positions: Record<string, PerpsPosition>;
  unlocks: PerpsUnlock[];
  reservedMargin: string;
  openOrderCount: number;
};

export type PerpsUserStateExtended = {
  raw: PerpsUserState;
  equity: string | null;
  availableMargin: string | null;
};

export type PerpsTimeInForce = "GTC" | "IOC" | "POST";

export type PerpsOrderKind =
  | { market: { maxSlippage: string } }
  | { limit: { limitPrice: string; timeInForce: PerpsTimeInForce } };

export type PerpsPairParam = {
  tickSize: string;
  minOrderSize: string;
  maxAbsOi: string;
  maxAbsFundingRate: string;
  initialMarginRatio: string;
  maintenanceMarginRatio: string;
  impactSize: string;
  vaultLiquidityWeight: string;
  vaultHalfSpread: string;
  vaultMaxQuoteSize: string;
  bucketSizes: string[];
};

export type PerpsPairState = {
  longOi: string;
  shortOi: string;
  fundingPerUnit: string;
  fundingRate: string;
};

export type RateSchedule = {
  base: string;
  tiers: Record<string, string>;
};

export type PerpsParam = {
  maxUnlocks: number;
  maxOpenOrders: number;
  liquidationBufferRatio: string;
  makerFeeRates: RateSchedule;
  takerFeeRates: RateSchedule;
  protocolFeeRate: string;
  liquidationFeeRate: string;
  fundingPeriod: number;
  vaultTotalWeight: string;
  vaultCooldownPeriod: number;
  referralActive: boolean;
  minReferrerVolume: string;
  referrerCommissionRates: RateSchedule;
};

export type PerpsState = {
  lastFundingTime: string;
  vaultShareSupply: string;
  insuranceFund: string;
  treasury: string;
};

export type PerpsVaultState = {
  shareSupply: string;
  equity: string;
  depositWithdrawalActive: boolean;
  margin: string;
  positions: Record<string, PerpsPosition>;
  reservedMargin: string;
  openOrderCount: number;
};

export type PerpsOrderResponse = {
  orderId: string;
  pairId: string;
  limitPrice: string;
  size: string;
  reduceOnly: boolean;
  reservedMargin: string;
};

export type PerpsOrderByUserItem = {
  pairId: string;
  size: string;
  limitPrice: string;
  reduceOnly: boolean;
  reservedMargin: string;
  createdAt: string;
};

export type PerpsOrdersByUserResponse = Record<string, PerpsOrderByUserItem>;

export type PerpsLiquidityDepth = {
  size: string;
  notional: string;
};

export type PerpsLiquidityDepthResponse = {
  bids: Record<string, PerpsLiquidityDepth>;
  asks: Record<string, PerpsLiquidityDepth>;
};

export type PerpsCancelOrderRequest = { one: string } | "all";

export type PerpsCancelConditionalOrderRequest =
  | { one: { pairId: string; triggerDirection: TriggerDirection } }
  | { allForPair: { pairId: string } }
  | "all";

export type PerpsQueryMsg =
  | { userState: { user: Address } }
  | { userStateExtended: { user: Address; includeEquity: boolean; includeAvailableMargin: boolean } }
  | { userStates: { startAfter?: Address; limit?: number } }
  | { param: Record<string, never> }
  | { pairParam: { pairId: string } }
  | { pairParams: { startAfter?: string; limit?: number } }
  | { state: Record<string, never> }
  | { pairState: { pairId: string } }
  | { pairStates: { startAfter?: string; limit?: number } }
  | { order: { orderId: string } }
  | { ordersByUser: { user: Address } }
  | { liquidityDepth: { pairId: string; bucketSize: string; limit?: number } }
  | { volume: { user: Address; since?: string } }
  | { vaultState: Record<string, never> };

export type GetPerpsQueryMsg<K extends KeyOfUnion<PerpsQueryMsg>> = ExtractFromUnion<
  PerpsQueryMsg,
  K
>;
