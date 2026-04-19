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
  realized_pnl: string;
  fee: string;
  /**
   * Identifier shared between the two `OrderFilled` events of a single
   * order-book match. `null` for trades executed before v0.15.0 — fill IDs
   * were not assigned prior to that release.
   */
  fill_id?: string | null;
};

export type LiquidatedData = {
  user: string;
  pair_id: string;
  adl_size: string;
  adl_price: string | null;
  adl_realized_pnl: string;
};

export type DeleveragedData = {
  user: string;
  pair_id: string;
  closing_size: string;
  fill_price: string;
  realized_pnl: string;
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
