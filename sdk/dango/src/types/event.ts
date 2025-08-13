import type { Hex, Json, UID } from "@left-curve/sdk/types";

export type IndexedEvent = {
  id: UID;
  parentId: UID;
  transactionId: UID;
  messageId: UID;
  type: string;
  method: string;
  eventStatus: EventStatus;
  commitmentStatus: CommitmentStatus;
  transactionType: number;
  transactionIdx: number;
  messageIdx: number;
  eventIdx: number;
  data: Json;
  blockHeight: number;
  createdAt: string;
  transaction: {
    hash: Hex;
  };
};

export type EventStatus = "ok" | "failed" | "nested_failed" | "handled";

export type CommitmentStatus = "committed" | "failed" | "reverted";
