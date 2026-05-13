import { HttpRequestError } from "../errors/request.js";
import { TimeoutError } from "../errors/timeout.js";
import type { HttpClientOptions, RpcClient } from "../types/index.js";
import { withTimeout } from "../utils/promises.js";

function createIdStore() {
  return {
    current: 0,
    take() {
      return this.current++;
    },
    reset() {
      this.current = 0;
    },
  };
}

export const idHandler = /*#__PURE__*/ createIdStore();

export function rpcClient(url: string, options: HttpClientOptions = {}): RpcClient {
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
                ? JSON.stringify(
                    body.map((body) => ({
                      ...body,
                      jsonrpc: "2.0",
                      id: body.id ?? idHandler.take(),
                    })),
                  )
                : JSON.stringify({
                    ...body,
                    jsonrpc: "2.0",
                    id: body.id ?? idHandler.take(),
                  }),
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
