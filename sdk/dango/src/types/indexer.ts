import type { Address, Json, UID } from "@left-curve/sdk/types";
import type { AccountTypes } from "./account.js";

export type IndexedBlock = {
  blockHeight: number;
  createdAt: string;
  hash: string;
  appHash: string;
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
  accountType: AccountTypes;
  createdAt: string;
  createdBlockHeight: number;
};

export type IndexedTransactionType = "CRON" | "TX";
