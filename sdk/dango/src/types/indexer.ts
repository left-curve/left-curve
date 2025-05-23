import type { Address } from "@left-curve/sdk/types";

export type IndexedBlock = {
  blockHeight: number;
  createdAt: string;
  hash: string;
  appHash: string;
  transactions: IndexedTransaction[];
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
  nestedEvents: string;
};

export type IndexedTransactionType = "CRON" | "TX";
