import type { MaybePromise } from "./utils.js";
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

export type GraphQLClientResponse<data = unknown> = {
  status: number;
  headers: Headers;
  data: data;
  extensions?: unknown;
  errors?: GraphQLError[];
};

export type GraphqlClientOptions = {
  fetchOptions?: Omit<RequestInit, "body">;
  onRequest?: (
    request: Request,
    init: RequestInit,
  ) => MaybePromise<void | undefined | (RequestInit & { url?: string | undefined })>;
  onResponse?: (response: Response) => Promise<void> | void;
  timeout?: number | undefined;
};

export type HttpRequestParameters<body = unknown> = GraphqlClientOptions & {
  body: body;
};

export type PageInfo = {
  hasNextPage: boolean;
  hasPreviousPage: boolean;
  startCursor?: string | null;
  endCursor?: string | null;
};

export type GraphqlPagination = {
  first?: number;
  last?: number;
  after?: string;
  before?: string;
  sortBy?: string;
};

export type GraphqlQueryResult<T> = {
  pageInfo: PageInfo;
  edge: {
    cursor: string;
    node: T;
  }[];
  nodes: T[];
};
