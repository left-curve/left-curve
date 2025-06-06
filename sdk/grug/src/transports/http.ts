import { UrlRequiredError } from "../errors/transports.js";
import { rpcClient } from "../http/rpcClient.js";
import { createTransport } from "./createTransport.js";

import { createBatchScheduler } from "../utils/scheduler.js";

import type {
  CometBftRpcSchema,
  HttpClientOptions,
  JsonRpcBatchOptions,
  JsonRpcRequest,
  Transport,
} from "../types/index.js";

export type HttpTransportConfig = {
  /**
   * Whether to enable Batch JSON-RPC.
   * @link https://www.jsonrpc.org/specification#batch
   */
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
  batch?: boolean | JsonRpcBatchOptions;
  /** The name of the transport. */
  name?: string;
  /** The key of the transport. */
  key?: string;
  /** The timeout (in ms) for the HTTP request. Default: 10_000 */
  timeout?: number;
};

export type HttpTransport = Transport<"http", CometBftRpcSchema>; /**
 * Creates a HTTP transport that connects to a JSON-RPC API.
 * @param url The URL of the JSON-RPC API.
 * @param config {HttpTransportConfig} The configuration of the transport.
 * @returns The HTTP transport.
 */
export function http(_url_?: string | undefined, config: HttpTransportConfig = {}): HttpTransport {
  const {
    key = "http",
    name = "HTTP JSON-RPC",
    batch: _batch_,
    timeout: _timeout_,
    fetchOptions,
    onFetchRequest,
    onFetchResponse,
  } = config;
  return ({ chain } = {}) => {
    const url = _url_ || chain?.urls.rpc;
    if (!url) throw new UrlRequiredError();

    const batchOptions = typeof _batch_ === "object" ? _batch_ : { maxSize: 20, maxWait: 20 };
    const batch = _batch_ ? batchOptions : undefined;
    const timeout = _timeout_ ?? 10_000;

    const client = rpcClient(url, {
      fetchOptions,
      onRequest: onFetchRequest,
      onResponse: onFetchResponse,
      timeout,
    });

    return createTransport<"http">({
      type: "http",
      name,
      key,
      async request({ method, params }) {
        const body = { method, params };

        const { schedule } = createBatchScheduler({
          id: url,
          wait: batchOptions.maxWait,
          shouldSplitBatch(requests) {
            return requests.length > batchOptions.maxSize;
          },
          fn: (body: JsonRpcRequest[]) => client.request({ body }),
          sort: (a, b) => a.id - b.id,
        });

        const fn = async (body: JsonRpcRequest) =>
          batch ? schedule(body) : [await client.request({ body })];

        const [{ error, result }] = await fn(body as JsonRpcRequest);

        if (error) {
          throw new Error(error.message);
        }
        return result;
      },
    });
  };
}
