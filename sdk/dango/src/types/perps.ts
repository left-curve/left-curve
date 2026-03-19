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

export type PerpsQueryMsg =
  | { userState: { user: Address } }
  | { userStates: { startAfter?: Address; limit?: number } };

export type GetPerpsQueryMsg<K extends KeyOfUnion<PerpsQueryMsg>> = ExtractFromUnion<
  PerpsQueryMsg,
  K
>;
