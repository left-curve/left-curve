import { createTransport } from "@left-curve/sdk";
import {
  HttpRequestError,
  UrlRequiredError,
  createBatchScheduler,
  wait,
} from "@left-curve/sdk/utils";
import { createClient } from "graphql-ws";
import { graphqlClient } from "../http/graphqlClient.js";
import { EventEmitter } from "eventemitter3";

import type {
  CometBftRpcSchema,
  HttpClientOptions,
  RequestFn,
  SubscribeFn,
  Transport,
} from "@left-curve/sdk/types";
import type { GraphQLClientResponse, GraphqlOperation } from "../types/graphql.js";

export type WsRetryConfig = {
  /** Maximum number of reconnection attempts before giving up. Default: 10 */
  maxRetries?: number;
  /** Initial delay in ms before first retry (doubles each attempt). Default: 1000 */
  baseDelay?: number;
  /** Maximum delay in ms between retries. Default: 30000 */
  maxDelay?: number;
};

export type GraphqlTransportConfig = {
  /**
   * Request configuration to pass to `fetch`.
   * @link https://developer.mozilla.org/en-US/docs/Web/API/fetch
   */
  fetchOptions?: HttpClientOptions["fetchOptions"];
  /** A callback to handle the response from `fetch`. */
  onFetchRequest?: HttpClientOptions["onRequest"];
  /** A callback to handle the response from `fetch`. */
  onFetchResponse?: HttpClientOptions["onResponse"];
  /** The batch configuration. */
  batch?: boolean;
  /** The name of the transport. */
  name?: string;
  /** The key of the transport. */
  key?: string;
  /** The timeout (in ms) for the HTTP request. Default: 10_000 */
  timeout?: number;
  /** Whether to create the WebSocket client in lazy mode. Default: true */
  lazy?: boolean;
  /** Disable WebSocket subscriptions entirely, forcing HTTP polling fallback. */
  disableWs?: boolean;
  /** When false, disables HTTP polling fallback for subscriptions. Default: true. */
  polling?: boolean;
  /** WebSocket retry configuration. */
  wsRetry?: WsRetryConfig;
};

export type GraphqlTransport = Transport<"http-graphql", CometBftRpcSchema>; /**
 * Creates a HTTP transport that connects to GraphQL API.
 * @param url The URL of the GraphQL API.
 * @param config {GraphqlTransportConfig} The configuration of the transport.
 * @returns The HTTP transport.
 */
export function graphql(
  _url_?: string | undefined,
  config: GraphqlTransportConfig = {},
): GraphqlTransport {
  const {
    key = "HTTP-Graphql",
    name = "HTTP Graphql",
    batch: _batch_,
    timeout: _timeout_,
    fetchOptions,
    onFetchRequest,
    onFetchResponse,
    lazy = true,
  } = config;

  const {
    maxRetries: wsMaxRetries = 10,
    baseDelay: wsBaseDelay = 1_000,
    maxDelay: wsMaxDelay = 30_000,
  } = config.wsRetry ?? {};

  // Shared WebSocket state across all transport factory invocations.
  // This ensures only one WS connection is created regardless of how many
  // clients (public, signer, etc.) use this transport.
  const wsClientStatus = { isConnected: false };
  const wsStatusEmitter = new EventEmitter();
  const wsRef: { current: ReturnType<typeof createClient> | null } = { current: null };
  let wsInitialized = false;

  const initWsClient = (url: string) => {
    if (wsInitialized) return;
    wsInitialized = true;

    const createWsClient = (isLazy: boolean) => {
      const ws = createClient({
        url,
        lazy: isLazy,
        keepAlive: 10_000,
        retryWait: async (retryCount: unknown) => {
          const count = typeof retryCount === "number" ? retryCount : 0;
          const delay = Math.min(wsBaseDelay * 2 ** count, wsMaxDelay);
          await wait(delay);
        },
        shouldRetry: (retryCount: unknown) =>
          typeof retryCount === "number" ? retryCount < wsMaxRetries : true,
      });

      ws.on("connected", () => {
        wsClientStatus.isConnected = true;
        wsStatusEmitter.emit("connected");
      });

      ws.on("closed", () => {
        wsClientStatus.isConnected = false;
        wsStatusEmitter.emit("closed");
      });

      return ws;
    };

    wsRef.current = createWsClient(lazy);

    const attemptReconnect = () => {
      if (!wsClientStatus.isConnected) {
        wsRef.current?.dispose();
        wsRef.current = createWsClient(false);
      }
    };

    if (typeof document !== "undefined") {
      document.addEventListener("visibilitychange", () => {
        if (document.visibilityState === "visible") attemptReconnect();
      });
    }

    if (typeof window !== "undefined") {
      window.addEventListener("online", attemptReconnect);
    }
  };

  return ({ chain } = {}) => {
    const url = _url_ || chain?.urls.indexer;
    if (!url) throw new UrlRequiredError();

    const batchOptions = typeof _batch_ === "object" ? _batch_ : { maxSize: 20, maxWait: 20 };
    const batch = _batch_ ? batchOptions : undefined;
    const timeout = _timeout_ ?? 10_000;

    if (!config.disableWs) {
      initWsClient(url);
    }

    const client = graphqlClient(url, {
      fetchOptions,
      onRequest: onFetchRequest,
      onResponse: onFetchResponse,
      timeout,
    });

    const request = (async ({ method, params }) => {
      const body = { query: method, variables: params || {} };

      const { schedule } = createBatchScheduler({
        id: url,
        wait: batchOptions.maxWait,
        shouldSplitBatch(requests) {
          return requests.length > batchOptions.maxSize;
        },
        fn: (body: GraphqlOperation[]) => client.request({ body }),
      });

      const fn = async (body: GraphqlOperation) =>
        batch ? schedule(body) : [await client.request({ body })];

      const [{ data, errors }] = (await fn(body as any)) as GraphQLClientResponse<
        CometBftRpcSchema[number]["ReturnType"]
      >[];

      if (errors?.length) {
        throw new HttpRequestError({
          body: body as Record<string, unknown>,
          details: errors.map((e) => e?.message ?? JSON.stringify(e)).join("; "),
          url,
        });
      }

      return data;
    }) as RequestFn<CometBftRpcSchema>;

    const noOp = () => {};

    const subscribe: SubscribeFn = ({ query, variables }, { next, error, complete }) => {
      if (config.disableWs) {
        return noOp;
      }
      return wsRef.current!.subscribe(
        { query, variables },
        {
          next: ({ data, errors }) => {
            if (errors) error?.(errors);
            if (data) next(data as any);
          },
          error: error || noOp,
          complete: complete || noOp,
        },
      );
    };

    subscribe.getClientStatus = () => (config.disableWs ? { isConnected: false } : wsClientStatus);
    subscribe.emitter = wsStatusEmitter;

    return createTransport<"http-graphql">({
      type: "http-graphql",
      name,
      key,
      batch: !!_batch_,
      polling: config.polling ?? true,
      request,
      subscribe,
    });
  };
}
