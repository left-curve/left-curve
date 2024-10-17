import type { MaybePromise, Prettify } from "./utils";

export type HttpRpcClientOptions = {
  /** Request configuration to pass to `fetch`. */
  fetchOptions?: Omit<RequestInit, "body">;
  /** A callback to handle the request. */
  onRequest?: (
    request: Request,
    init: RequestInit,
  ) => MaybePromise<void | undefined | (RequestInit & { url?: string | undefined })>;
  /** A callback to handle the response. */
  onResponse?: (response: Response) => Promise<void> | void;
  /** The timeout (in ms) for the request. */
  timeout?: number | undefined;
};

export type HttpRequestParameters<body = unknown> = Prettify<
  HttpRpcClientOptions & {
    /** The RPC request body. */
    body: body;
  }
>;
