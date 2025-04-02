import { HttpRequestError, TimeoutError, withTimeout } from "@left-curve/sdk/utils";
import { GraphQLClient } from "graphql-request";

import type { Json } from "@left-curve/sdk/types";
import type { GraphqlClient, GraphqlClientOptions } from "../types/graphql.js";

export function graphqlClient(url: string, options: GraphqlClientOptions = {}): GraphqlClient {
  const { fetchOptions, onRequest, onResponse, timeout = 10_000 } = options;

  const client = new GraphQLClient(url, {
    fetch: async (_, requestInit) => {
      const init = requestInit ?? {};
      const { body } = init;
      const { headers, method, signal: signal_ } = fetchOptions || {};

      try {
        const response = await withTimeout(
          async ({ signal }) => {
            const request = new Request(url, {
              ...init,
              headers: {
                "Content-Type": "application/json",
                ...headers,
              },
              method: method || "POST",
              signal: signal_ || (timeout > 0 ? signal : null),
            });
            const args = (await onRequest?.(request, init)) ?? { ...init, url };
            return fetch(args.url ?? url, args);
          },
          {
            errorInstance: new TimeoutError({ body: { body }, url }),
            timeout,
            signal: true,
          },
        );

        if (onResponse) await onResponse(response);

        return response;
      } catch (err) {
        if (err instanceof HttpRequestError) throw err;
        if (err instanceof TimeoutError) throw err;
        throw new HttpRequestError({
          cause: err as Error,
          body: { body },
          url,
        });
      }
    },
  });
  return {
    async request<response = unknown, variables = Json>(
      document: string,
      variables: variables,
    ): Promise<response> {
      const response = await client.rawRequest(document, variables ?? {});
      if (response.errors) {
        const [{ message }] = response.errors;
        throw new Error(message);
      }
      return response.data as response;
    },
  };
}
