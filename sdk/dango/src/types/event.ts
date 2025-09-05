import type { Address, Coins, Denom, Hex, Json, UID } from "@left-curve/sdk/types";
import type { Directions, OrderId, OrderTypes } from "./dex.js";

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
  data: EventData;
  blockHeight: number;
  createdAt: string;
  transaction: {
    hash: Hex;
  };
};

export type ContractEvent = {
  contract_event: {
    contract: Address;
    data: Json;
    type: string;
  };
};

export type ExecuteEvent = {
  execute: {
    contract: Address;
    execute_msg: Json;
    funds: Coins;
    sender: Address;
  };
};

export type TransferEvent = {
  transfer: {
    sender: Address;
    transfers: Record<Address, Coins>;
  };
};

export type OrderCreatedEvent = {
  amount: string;
  base_denom: Denom;
  deposit: {
    amount: string;
    denom: Denom;
  };
  direction: Directions;
  id: OrderId;
  kind: OrderTypes;
  price: string;
  quote_denom: Denom;
  user: Address;
};

export type OrderCanceledEvent = {
  id: OrderId;
  kind: OrderTypes;
  user: Address;
  remaining: string;
  direction: Directions;
  base_denom: Denom;
  quote_denom: Denom;
  price: string;
  amount: string;
  refund: {
    amount: string;
    denom: Denom;
  };
};

export type OrderFilledEvent = {
  user: Address;
  id: OrderId;
  kind: OrderTypes;
  base_denom: Denom;
  quote_denom: Denom;
  direction: Directions;
  filled_base: string;
  filled_quote: string;
  refund_base: string;
  refund_quote: string;
  fee_base: string;
  fee_quote: string;
  clearing_price: string;
  remaining: string;
  cleared: boolean;
};

export type EventData = ContractEvent | ExecuteEvent | TransferEvent;

export type EventStatus = "ok" | "failed" | "nested_failed" | "handled";

export type CommitmentStatus = "committed" | "failed" | "reverted";
