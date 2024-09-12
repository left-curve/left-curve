import type {
  Json,
  JsonRpcBatchOptions,
  JsonRpcRequest,
  JsonRpcSuccessResponse,
  RpcClient,
} from "@leftcurve/types";

export function httpRpc(
  endpoint: string,
  headers: Record<string, string | string[]> = {},
  batchOptions?: JsonRpcBatchOptions,
): RpcClient {
  const queue: {
    request: JsonRpcRequest;
    resolve: (a: JsonRpcSuccessResponse<unknown>) => void;
    reject: (a: Error) => void;
  }[] = [];

  const useBach = batchOptions !== undefined;

  let timer: ReturnType<typeof setInterval> | undefined;

  const flush = async () => {
    if (batchOptions === undefined) return;
    const batch = queue.splice(0, batchOptions.maxSize);

    if (batch.length === 0 && timer) {
      clearInterval(timer);
      timer = undefined;
      return;
    }

    try {
      const requests = batch.map((item) => item.request);
      const requestMap = new Map(batch.map((item) => [item.request.id, item]));

      const responses = await handleRequest(requests);
      const responsesArray = Array.isArray(responses) ? responses : [responses];

      for (const response of responsesArray) {
        const request = requestMap.get(response.id);
        if (!request) continue;
        if ("error" in response) {
          request.reject(response.error);
        } else {
          request.resolve(response);
        }
      }
    } catch (err) {
      for (const item of batch) {
        item.reject(err instanceof Error ? err : new Error("RPC Client: something went wrong"));
      }
    }
  };

  const handleRequest = async (request: JsonRpcRequest | JsonRpcRequest[]) => {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: {
        ...headers,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(request),
    });
    return await response.json();
  };

  return {
    request: async (method: string, params: Json) => {
      const request: JsonRpcRequest = {
        id: Date.now(),
        jsonrpc: "2.0",
        method,
        params,
      };

      if (!useBach) return await handleRequest(request);

      const promise = new Promise<JsonRpcSuccessResponse<unknown>>((resolve, reject) => {
        queue.push({ request, resolve, reject });
        if (queue.length && timer) {
          flush();
        } else if (!timer) {
          timer = setInterval(flush, batchOptions.maxWait);
        }
      });

      return await promise;
    },
  };
}
