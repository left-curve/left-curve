import type {
  HttpRequestParameters,
  JsonValue,
  MaybePromise,
  TransportSchemaOverride,
} from "@left-curve/sdk/types";
import type { GraphQLError } from "graphql";

export type GraphqlOperation<variables extends object | undefined = undefined> = {
  query: string;
  variables: variables;
};

export type GraphqlClient = {
  readonly request: <
    response,
    variables extends object | undefined,
    body extends GraphqlOperation<variables> | GraphqlOperation<variables>[],
  >(
    params: HttpRequestParameters<body>,
  ) => Promise<body extends GraphqlOperation<variables> ? response : response[]>;
};
export interface GraphQLSchemaOverride<T = JsonValue> extends TransportSchemaOverride {
  Method: string;
  Parameters?: Record<string, unknown>;
  ReturnType: T;
}

export type GraphQLClientResponse<data = unknown> = {
  status: number;
  headers: Headers;
  data: data;
  extensions?: unknown;
  errors?: GraphQLError[];
};

export type GraphqlClientOptions = {
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
