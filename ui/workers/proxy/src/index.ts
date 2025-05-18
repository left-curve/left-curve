interface Env {
  SERVER_URI: string;
}

const getPort = (pathname: string) => {
  if (pathname.includes("/rpc")) return "26657";
  if (pathname.includes("/graphql")) return "8080";
  if (pathname.includes("/quests")) return "8081";
  if (pathname.includes("/faucet")) return "8082";
  throw new Error("Invalid path");
};

const withTimeout = async (promise: Promise<Response>, timeout: number) => {
  // biome-ignore lint/suspicious/noAsyncPromiseExecutor: we need to use async executor to handle timeout
  return await new Promise<Response>(async (resolve) => {
    setTimeout(() => resolve({ ok: false } as Response), timeout);
    const response = await promise;
    resolve(response);
  });
};

export default {
  async fetch(request: Request, env: Env) {
    if (request.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
          "Access-Control-Allow-Headers":
            request.headers.get("Access-Control-Request-Headers") || "*",
          "Access-Control-Max-Age": "86400",
        },
      });
    }

    if (request.headers.get("Upgrade") === "websocket") {
      const wsRequest = new Request(`http://${env.SERVER_URI}:8080/graphql`, {
        method: request.method,
        headers: request.headers,
        body: request.body,
      });
      return await fetch(wsRequest);
    }

    const url = new URL(request.url);

    if (
      url.pathname.includes("/rpc") ||
      url.pathname.includes("/quests") ||
      url.pathname.includes("/graphql") ||
      url.pathname.includes("/faucet")
    ) {
      const PORT = getPort(url.pathname);
      const PROTOCOL = url.protocol.includes("http") ? "http" : "ws";

      const newRequest = new Request(
        `${PROTOCOL}://${env.SERVER_URI}:${PORT}${url.pathname.replace("/rpc", "").replace("/quests", "").replace("/faucet", "")}${url.search}`,
        {
          method: request.method,
          headers: request.headers,
          body: request.body,
          redirect: "follow",
        },
      );

      const response = await fetch(newRequest);

      return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers: {
          "Content-Type": response.headers.get("Content-Type") ?? "application/json",
          "Access-Control-Allow-Origin": request.headers.get("Origin") ?? "*",
          "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
        },
      });
    }

    const [questsStatus, graphqlStatus, rpcStatus, faucetStatus] = await Promise.all([
      withTimeout(fetch(`http://${env.SERVER_URI}:8081/check_username/none`), 500),
      withTimeout(fetch(`http://${env.SERVER_URI}:8080`), 500),
      withTimeout(fetch(`http://${env.SERVER_URI}:26657`), 500),
      withTimeout(fetch(`http://${env.SERVER_URI}:8082/health`), 500),
    ]);

    return new Response(
      JSON.stringify({
        health: {
          quests: questsStatus.ok ? "up" : "down",
          graphql: graphqlStatus.ok ? "up" : "down",
          rpc: rpcStatus.ok ? "up" : "down",
          faucet: faucetStatus.ok ? "up" : "down",
        },
      }),
      {
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
          "Access-Control-Allow-Headers":
            request.headers.get("Access-Control-Request-Headers") || "*",
          "Access-Control-Max-Age": "86400",
        },
        status: 200,
      },
    );
  },
};
