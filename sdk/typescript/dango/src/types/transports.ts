import type { EventEmitter } from "eventemitter3";
import type { Chain } from "./chain.js";

export type RequestFn = <T = unknown>(args: {
  request: string;
  params?: Record<string, unknown>;
}) => Promise<T>;

export type SubscribeFn = (<T>(
  { query, variables }: { query: string; variables?: Record<string, unknown> },
  callback: SubscriptionCallbacks<T>,
) => () => void) & { getClientStatus?: () => { isConnected: boolean }; emitter?: EventEmitter };

export type SubscriptionCallbacks<T = unknown> = {
  next: (data: T) => void;
  error?: (error: unknown) => void;
  complete?: () => void;
};

export type RequestOptions = {
  dedupe?: boolean | undefined;
  retryDelay?: number | undefined;
  retryCount?: number | undefined;
  uid?: string | undefined;
};

export type Transport = (chain: Chain) => {
  request: RequestFn;
  subscribe: SubscribeFn;
};
