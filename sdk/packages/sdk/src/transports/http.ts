import { UrlRequiredError } from "~/errors/transports";
import { httpRpc } from "~/rpc/httpClient";
import { createTransport } from "./createTransport";

import type {
  CometBftRpcSchema,
  HttpRpcClientOptions,
  JsonRpcBatchOptions,
  JsonRpcRequest,
  RequestFn,
  Transport,
} from "@leftcurve/types";
import { createBatchScheduler } from "@leftcurve/utils";

export type HttpTransportConfig = {
  /**
   * Whether to enable Batch JSON-RPC.
   * @link https://www.jsonrpc.org/specification#batch
   */
  /**
   * Request configuration to pass to `fetch`.
   * @link https://developer.mozilla.org/en-US/docs/Web/API/fetch
   */
  fetchOptions?: HttpRpcClientOptions["fetchOptions"];
  /** A callback to handle the response from `fetch`. */
  onFetchRequest?: HttpRpcClientOptions["onRequest"];
  /** A callback to handle the response from `fetch`. */
  onFetchResponse?: HttpRpcClientOptions["onResponse"];
  /** The batch configuration. */
  batch?: boolean | JsonRpcBatchOptions;
  /** The name of the transport. */
  name?: string;
  /** The headers of the transport. */
  headers?: Record<string, string | string[]>;
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
    const url = _url_ || chain?.rpcUrls.default.http[0];
    if (!url) throw new UrlRequiredError();

    const batchOptions = typeof _batch_ === "object" ? _batch_ : { maxSize: 20, maxWait: 20 };
    const batch = _batch_ ? batchOptions : undefined;
    const timeout = _timeout_ ?? 10_000;

    const rpcClient = httpRpc(url, {
      fetchOptions,
      onRequest: onFetchRequest,
      onResponse: onFetchResponse,
      timeout,
    });

    const request = (async ({ method, params }) => {
      const body = { method, params };

      const { schedule } = createBatchScheduler({
        id: url,
        wait: batchOptions.maxWait,
        shouldSplitBatch(requests) {
          return requests.length > batchOptions.maxSize;
        },
        fn: (body: JsonRpcRequest[]) => rpcClient.request({ body }),
        sort: (a, b) => a.id - b.id,
      });

      const fn = async (body: JsonRpcRequest) =>
        batch ? schedule(body) : [await rpcClient.request({ body })];

      const [{ error, result }] = await fn(body as JsonRpcRequest);

      if (error) {
        throw new Error(error.message);
      }
      return result;
    }) as RequestFn<CometBftRpcSchema>;

    return createTransport({
      type: "http",
      name,
      key,
      request,
    });
  };
}
