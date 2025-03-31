import { createTransport } from "@left-curve/sdk";
import { UrlRequiredError } from "@left-curve/sdk/utils";
import { graphqlClient } from "../http/graphqlClient.js";

import type {
  CometBftRpcSchema,
  HttpRpcClientOptions,
  RequestFn,
  Transport,
} from "@left-curve/sdk/types";

export type GraphqlTransportConfig = {
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
  batch?: boolean;
  /** The name of the transport. */
  name?: string;
  /** The key of the transport. */
  key?: string;
  /** The timeout (in ms) for the HTTP request. Default: 10_000 */
  timeout?: number;
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
  } = config;
  return ({ chain } = {}) => {
    const url = _url_ || chain?.urls.indexer;
    if (!url) throw new UrlRequiredError();

    const batchOptions = typeof _batch_ === "object" ? _batch_ : { maxSize: 20, maxWait: 20 };
    const batch = _batch_ ? batchOptions : undefined;
    const timeout = _timeout_ ?? 10_000;

    const client = graphqlClient(url, {
      fetchOptions,
      onRequest: onFetchRequest,
      onResponse: onFetchResponse,
      timeout,
    });

    const request: RequestFn<CometBftRpcSchema> = async ({ method, params }) => {
      return client.request(method, params);
    };

    return createTransport<"http-graphql">({
      type: "http-graphql",
      name,
      key,
      request,
    });
  };
}
