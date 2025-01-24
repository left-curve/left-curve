import type { Account, Coins, Prettify } from "@left-curve/dango/types";

export const EventType = {
  Balance: "balances",
  Account: "account",
} as const;

export type EventsType = (typeof EventType)[keyof typeof EventType];

type BaseEvent<event> = Prettify<{ timestamp: number } & event>;

export type EventBalance = BaseEvent<{ balances: Coins; type: typeof EventType.Balance }>;
export type EventAccount = BaseEvent<{ account: Account; type: typeof EventType.Account }>;

export type Events = EventBalance | EventAccount;
