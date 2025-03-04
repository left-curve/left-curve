import { gql } from "graphql-request";
import { UrlRequiredError } from "../errors/transports.js";
import { createTransport } from "./createTransport.js";

import { encodeBase64 } from "../encoding/base64.js";
import { serialize } from "../encoding/binary.js";
import { graphqlClient } from "../http/graphqlClient.js";

import type {
  ChainStatusResponse,
  HttpRpcClientOptions,
  IndexerSchema,
  QueryResponse,
  RequestFn,
  Transport,
} from "../types/index.js";
import { recursiveTransform } from "../utils/mappers.js";
import { camelToSnake, snakeToCamel } from "../utils/strings.js";

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

export type GraphqlTransport = Transport<"http-grahpql", IndexerSchema>; /**
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

    const request = (async ({
      method,
      params,
    }: {
      method: IndexerSchema[number]["Method"];
      params: IndexerSchema[number]["Parameters"];
    }): Promise<IndexerSchema[number]["ReturnType"]> => {
      switch (method) {
        case "query_app": {
          const { query, height, prove } = params as IndexerSchema[0]["Parameters"];
          const document = gql`
            query queryApp($request: String!, $height: Int!, $prove: Boolean!) {
              queryApp(request: $request, height: $height, prove: $prove)
            }
          `;

          const { queryApp } = await client.request<{ queryApp: string }>(document, {
            request: JSON.stringify(recursiveTransform(query, camelToSnake)),
            height,
            prove,
          });

          return recursiveTransform<IndexerSchema[0]["ReturnType"]>(
            JSON.parse(queryApp),
            snakeToCamel,
          );
        }
        case "query_status": {
          const document = gql`
            query {
              queryStatus {
                chainId
                block {
                  blockHeight
                  timestamp
                  hash
                }
              }
            }
          `;

          const { queryStatus } = await client.request<{ queryStatus: ChainStatusResponse }>(
            document,
          );

          return queryStatus as IndexerSchema[1]["ReturnType"];
        }
        case "simulate": {
          const { tx, height, prove } = params as IndexerSchema[2]["Parameters"];
          const document = gql`
            query simulate($tx: String!, $height: Int!, $prove: Boolean! = false)  {
              simulate(tx: $tx, height: $height, prove: $prove)
            }
          `;

          const { simulate } = await client.request<{ simulate: string }>(document, {
            tx: JSON.stringify(recursiveTransform(tx, camelToSnake)),
            height,
            prove,
          });

          return recursiveTransform<IndexerSchema[2]["ReturnType"]>(
            JSON.parse(simulate),
            snakeToCamel,
          );
        }
        case "broadcast": {
          const { tx, mode } = params as IndexerSchema[3]["Parameters"];
          const document = gql`
              mutation broadcastTxSync($tx: String!) {
                  broadcastTxSync(tx: $tx) {
                    hash
                    log
                    code
                  }
                }
            `;
          const { broadcastTxSync } = await client.request<{
            broadcastTxSync: IndexerSchema[3]["ReturnType"];
          }>(document, {
            tx: encodeBase64(serialize(tx)),
          });
          const { code } = broadcastTxSync;

          if (code === 0) return broadcastTxSync;

          throw new Error(
            `failed to broadcast tx! codespace: ${broadcastTxSync.codespace}, code: ${code}, log: ${broadcastTxSync.log}`,
          );
        }
        default:
          throw new Error(`Method ${method} not supported`);
      }
    }) as RequestFn<IndexerSchema>;

    return createTransport<"http-grahpql">({
      type: "http-grahpql",
      name,
      key,
      request,
    });
  };
}
