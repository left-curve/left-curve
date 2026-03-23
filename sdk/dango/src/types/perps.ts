import type { Address, ExtractFromUnion, KeyOfUnion } from "@left-curve/sdk/types";

export type PerpsPosition = {
  size: string;
  entryPrice: string;
  entryFundingPerUnit: string;
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

export type PerpsOrderKind =
  | { market: { maxSlippage: string } }
  | { limit: { limitPrice: string; postOnly: boolean } };

export type TriggerDirection = "above" | "below";

export type LimitOrConditionalOrder =
  | { limit: { limitPrice: string; reduceOnly: boolean; reservedMargin: string } }
  | { conditional: { triggerPrice: string; triggerDirection: TriggerDirection } };

export type PerpsOrdersByUserResponseItem = {
  pairId: string;
  size: string;
  kind: LimitOrConditionalOrder;
  createdAt: string;
};

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
};

export type PerpsParam = {
  maxUnlocks: number;
  maxOpenOrders: number;
  baseMakerFeeRate: string;
  baseTakerFeeRate: string;
  tieredMakerFeeRate: Record<string, string>;
  tieredTakerFeeRate: Record<string, string>;
  protocolFeeRate: string;
  liquidationFeeRate: string;
  fundingPeriod: number;
  vaultTotalWeight: string;
  vaultCooldownPeriod: number;
};

export type PerpsState = {
  lastFundingTime: string;
  vaultShareSupply: string;
  insuranceFund: string;
  treasury: string;
};

export type PerpsOrderResponse = {
  orderId: string;
  pairId: string;
  limitPrice: string;
  size: string;
  reduceOnly: boolean;
  reservedMargin: string;
};

export type PerpsOrdersByUserResponse = Record<string, PerpsOrdersByUserResponseItem>;

export type PerpsLiquidityDepth = {
  size: string;
  notional: string;
};

export type PerpsLiquidityDepthResponse = {
  bids: Record<string, PerpsLiquidityDepth>;
  asks: Record<string, PerpsLiquidityDepth>;
};

export type PerpsCancelOrderRequest = { one: string } | "all";

export type PerpsQueryMsg =
  | { userState: { user: Address } }
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
  | { volume: { user: Address; since?: string } };

export type GetPerpsQueryMsg<K extends KeyOfUnion<PerpsQueryMsg>> = ExtractFromUnion<
  PerpsQueryMsg,
  K
>;
