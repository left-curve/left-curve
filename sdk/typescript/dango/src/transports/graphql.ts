import type {
  GraphQLClientResponse,
  GraphqlOperation,
  RequestFn,
  SubscribeFn,
  Transport,
} from "@left-curve/types";
import { createBatchScheduler, wait } from "@left-curve/utils";
import { EventEmitter } from "eventemitter3";
import { createClient } from "graphql-ws";
import { HttpRequestError } from "#errors/request.js";
import { UrlRequiredError } from "#errors/transports.js";
import { graphqlClient } from "#http/graphqlClient.js";

export type WsRetryConfig = {
  maxRetries?: number;
  baseDelay?: number;
  maxDelay?: number;
};

export type GraphqlTransportConfig = {
  fetchOptions?: Omit<RequestInit, "body">;
  onFetchRequest?: (
    request: Request,
    init: RequestInit,
  ) =>
    | Promise<void | undefined | (RequestInit & { url?: string | undefined })>
    | void
    | undefined
    | (RequestInit & { url?: string | undefined });
  onFetchResponse?: (response: Response) => Promise<void> | void;
  batch?: boolean;
  name?: string;
  key?: string;
  timeout?: number;
  lazy?: boolean;
  disableWs?: boolean;
  polling?: boolean;
  wsRetry?: WsRetryConfig;
};

export function createTransport(
  _url_?: string | undefined,
  config: GraphqlTransportConfig = {},
): Transport {
  const {
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

  return (chain) => {
    const url = _url_ || chain.url;
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

    const request = (async ({ request: query, params }) => {
      const body = { query, variables: params || {} };

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

      const [{ data, errors }] = (await fn(body as any)) as GraphQLClientResponse[];

      if (errors?.length) {
        throw new HttpRequestError({
          body: body as Record<string, unknown>,
          details: errors.map((e) => e?.message ?? JSON.stringify(e)).join("; "),
          url,
        });
      }

      return data;
    }) as RequestFn;

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

    return {
      request,
      subscribe,
    };
  };
}
