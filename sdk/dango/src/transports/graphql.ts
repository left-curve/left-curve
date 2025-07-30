import { createTransport } from "@left-curve/sdk";
import { UrlRequiredError, createBatchScheduler } from "@left-curve/sdk/utils";
import { createClient } from "graphql-ws";
import { graphqlClient } from "../http/graphqlClient.js";

import type {
  CometBftRpcSchema,
  HttpClientOptions,
  RequestFn,
  SubscribeFn,
  Transport,
} from "@left-curve/sdk/types";
import type { GraphQLClientResponse, GraphqlOperation } from "#types/graphql.js";

export type GraphqlTransportConfig = {
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

    const wsClient = createClient({ url });

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

      const [{ data }] = (await fn(body as any)) as GraphQLClientResponse<
        CometBftRpcSchema[number]["ReturnType"]
      >[];

      return data;
    }) as RequestFn<CometBftRpcSchema>;

    const noOp = () => {};

    const subscribe: SubscribeFn = ({ query, variables }, { next, error, complete }) => {
      return wsClient.subscribe(
        { query, variables },
        {
          next: ({ data }) => data && next(data as any),
          error: error || noOp,
          complete: complete || noOp,
        },
      );
    };

    subscribe(
      {
        query: `
        subscription {
          eventByAddresses(
            addresses: ["0x6e8fdeefaa7b8fb1f559e6e944050cdbeb0f4358"]
          ) {
            data
            type
          }
        }
        `,
      },
      {
        next: (data) => {
          console.log("events received", data);
        },
        error: (error) => {
          console.error(error);
        },
        complete: () => {
          console.log("complete");
        },
      },
    );

    return createTransport<"http-graphql">({
      type: "http-graphql",
      name,
      key,
      request,
      subscribe,
    });
  };
}
