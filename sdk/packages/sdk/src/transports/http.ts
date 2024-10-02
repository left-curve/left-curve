import { UrlRequiredError } from "~/errors/transports";
import { httpRpc } from "~/rpc/httpClient";
import { createTransport } from "./createTransport";

import type {
  CometBftRpcSchema,
  Json,
  JsonRpcBatchOptions,
  RequestFn,
  Transport,
} from "@leftcurve/types";

export type HttpTransportConfig = {
  /**
   * Whether to enable Batch JSON-RPC.
   * @link https://www.jsonrpc.org/specification#batch
   */
  batch?: boolean | JsonRpcBatchOptions;
  /** The name of the transport. */
  name?: string;
  /** The headers of the transport. */
  headers?: Record<string, string | string[]>;
  /** The key of the transport. */
  key?: string;
};

export type HttpTransport = Transport<"http", CometBftRpcSchema>; /**
 * Creates a HTTP transport that connects to a JSON-RPC API.
 * @param url The URL of the JSON-RPC API.
 * @param config {HttpTransportConfig} The configuration of the transport.
 * @returns The HTTP transport.
 */
export function http(_url_?: string | undefined, config: HttpTransportConfig = {}): HttpTransport {
  const { key = "http", name = "HTTP JSON-RPC", batch: _batch_, headers } = config;
  return ({ chain } = {}) => {
    const url = _url_ || chain?.rpcUrls.default.http[0];
    if (!url) throw new UrlRequiredError();

    const batchOptions = typeof _batch_ === "object" ? _batch_ : { maxSize: 20, maxWait: 20 };
    const batch = _batch_ ? batchOptions : undefined;

    const rpcClient = httpRpc(url, headers, batch);

    const request = (async ({ method, params }, options) => {
      const { result, error } = await rpcClient.request(method, params as Json);

      if (error) throw new Error(error.message);

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
