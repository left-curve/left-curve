import type { Address, Hex, Json, JsonString, UID } from "@left-curve/sdk/types";

export type IndexedBlock = {
  blockHeight: number;
  createdAt: string;
  hash: string;
  appHash: string;
  cronsOutcomes: JsonString;
  transactions: IndexedTransaction[];
};

export type IndexedTradeSideType = "BUY" | "SELL";

export type IndexedTrade = {
  price: string;
  size: string;
  createdAt: string;
  hash: string;
  side: IndexedTradeSideType;
};

export type IndexedTransaction = {
  blockHeight: number;
  createdAt: string;
  transactionType: IndexedTransactionType;
  transactionIdx: number;
  sender: Address;
  hash: string;
  hasSucceeded: boolean;
  errorMessage: string;
  gasWanted: number;
  gasUsed: number;
  messages: IndexedMessage[];
  nestedEvents: string;
};

export type IndexedMessage = {
  methodName: string;
  blockHeight: number;
  contractAddr: Address;
  senderAddr: Address;
  orderIdx: number;
  createdAt: string;
  data: Record<string, Json>;
};

export type IndexedTransferEvent = {
  id: UID;
  txHash: Hex;
  fromAddress: Address;
  toAddress: Address;
  createdAt: string;
  blockHeight: number;
  amount: string;
  denom: string;
};

export type IndexedAccountEvent = {
  id: UID;
  accountIndex: number;
  address: Address;
  createdAt: string;
  createdBlockHeight: number;
};

export type IndexedTransactionType = "CRON" | "TX";

export type PerpsTrade = {
  orderId: string;
  pairId: string;
  user: string;
  fillPrice: string;
  fillSize: string;
  closingSize: string;
  openingSize: string;
  realizedPnl: string;
  fee: string;
  createdAt: string;
  blockHeight: number;
  tradeIdx: number;
  /**
   * Identifier shared between the two `OrderFilled` events of a single
   * order-book match. `null` for trades executed before v0.15.0 — fill IDs
   * were not assigned prior to that release.
   */
  fillId?: string | null;
  /**
   * `true` for the maker side of a match, `false` for the taker side.
   * `null` for trades executed before v0.16.0 — the maker/taker flag was
   * not recorded prior to that release.
   */
  isMaker?: boolean | null;
};

export type PerpsEventType = "order_filled" | "liquidated" | "deleveraged";

export type OrderFilledData = {
  order_id: string;
  pair_id: string;
  user: string;
  fill_price: string;
  fill_size: string;
  closing_size: string;
  opening_size: string;
  /**
   * Closing PnL on the fill (price movement on the closed portion).
   * Prior to v0.17.0 (exclusive) this also bundled the funding settled
   * on the user's pre-existing position; from v0.17.0 (inclusive)
   * onward, that funding component is reported separately as
   * `realized_funding`.
   */
  realized_pnl: string;
  /**
   * Funding settled on the user's pre-existing position immediately
   * before this fill. `null` for trades executed before v0.17.0 —
   * funding was reported as part of `realized_pnl` prior to that
   * release.
   */
  realized_funding?: string | null;
  fee: string;
  /**
   * Identifier shared between the two `OrderFilled` events of a single
   * order-book match. `null` for trades executed before v0.15.0 — fill IDs
   * were not assigned prior to that release.
   */
  fill_id?: string | null;
  /**
   * `true` for the maker side of a match, `false` for the taker side.
   * `null` for trades executed before v0.16.0 — the maker/taker flag was
   * not recorded prior to that release.
   */
  is_maker?: boolean | null;
};

export type LiquidatedData = {
  user: string;
  pair_id: string;
  adl_size: string;
  adl_price: string | null;
  /**
   * Closing PnL realized by the liquidated user from ADL fills,
   * accumulated across all counter-party fills for this pair. Prior to
   * v0.17.0 (exclusive) this also bundled the funding settled on the
   * user's position immediately before each ADL fill; from v0.17.0
   * (inclusive) onward, that funding component is reported separately
   * as `adl_realized_funding`.
   */
  adl_realized_pnl: string;
  /**
   * Funding settled on the liquidated user's position immediately
   * before each ADL fill, accumulated across all counter-party fills
   * for this pair. `null` for liquidations executed before v0.17.0 —
   * funding was reported as part of `adl_realized_pnl` prior to that
   * release.
   */
  adl_realized_funding?: string | null;
};

export type DeleveragedData = {
  user: string;
  pair_id: string;
  closing_size: string;
  fill_price: string;
  /**
   * Closing PnL realized by the counter-party from this ADL fill.
   * Prior to v0.17.0 (exclusive) this also bundled the funding settled
   * on the counter-party's pre-existing position immediately before
   * the fill; from v0.17.0 (inclusive) onward, that funding component
   * is reported separately as `realized_funding`.
   */
  realized_pnl: string;
  /**
   * Funding settled on the counter-party's pre-existing position
   * immediately before this ADL fill. `null` for ADL fills executed
   * before v0.17.0 — funding was reported as part of `realized_pnl`
   * prior to that release.
   */
  realized_funding?: string | null;
};

export type PerpsEvent = {
  idx: number;
  blockHeight: number;
  txHash: string;
  eventType: PerpsEventType;
  userAddr: string;
  pairId: string;
  data: OrderFilledData | LiquidatedData | DeleveragedData;
  createdAt: string;
};
