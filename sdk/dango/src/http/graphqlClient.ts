import { HttpRequestError, TimeoutError, withTimeout } from "@left-curve/sdk/utils";

import type { GraphqlClient, GraphqlClientOptions } from "../types/graphql.js";

export function graphqlClient(url: string, options: GraphqlClientOptions = {}): GraphqlClient {
  return {
    async request(params) {
      const {
        body,
        onRequest = options.onRequest,
        onResponse = options.onResponse,
        timeout = options.timeout ?? 10_000,
      } = params;

      const fetchOptions = {
        ...(options.fetchOptions ?? {}),
        ...(params.fetchOptions ?? {}),
      };

      const { headers, method, signal: signal_ } = fetchOptions;

      try {
        const response = await withTimeout(
          async ({ signal }) => {
            const init: RequestInit = {
              ...fetchOptions,
              body: Array.isArray(body)
                ? JSON.stringify(body.map((body) => ({ ...body })))
                : JSON.stringify({ ...body }),
              headers: {
                "Content-Type": "application/json",
                ...headers,
              },
              method: method || "POST",
              signal: signal_ || (timeout > 0 ? signal : null),
            };
            const request = new Request(url, init);
            const args = (await onRequest?.(request, init)) ?? { ...init, url };
            const response = await fetch(args.url ?? url, args);
            return response;
          },
          {
            errorInstance: new TimeoutError({ body: body, url }),
            timeout,
            signal: true,
          },
        );

        if (onResponse) await onResponse(response);

        let data: any;
        if (response.headers.get("Content-Type")?.startsWith("application/json"))
          data = await response.json();
        else {
          data = await response.text();
          try {
            data = JSON.parse(data || "{}");
          } catch (err) {
            if (response.ok) throw err;
            data = { error: data };
          }
        }

        if (!response.ok) {
          throw new HttpRequestError({
            body,
            details: JSON.stringify(data.error) || response.statusText,
            headers: response.headers,
            status: response.status,
            url,
          });
        }

        const error = Array.isArray(data) ? data.at(0)?.errors?.at(0) : data.errors?.at(0);

        if (error) {
          throw new HttpRequestError({
            body,
            details: JSON.stringify(error.message),
            headers: response.headers,
            status: response.status,
            url,
          });
        }

        return data;
      } catch (err) {
        if (err instanceof HttpRequestError) throw err;
        if (err instanceof TimeoutError) throw err;
        throw new HttpRequestError({
          body,
          cause: err as Error,
          url,
        });
      }
    },
  };
}
