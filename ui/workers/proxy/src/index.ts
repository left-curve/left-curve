interface Env {
  SERVER_URI: string;
}

const getPort = (pathname: string) => {
  if (pathname.includes("/rpc")) return "26657";
  if (pathname.includes("/graphql")) return "8080";
  if (pathname.includes("/quests")) return "8081";
  throw new Error("Invalid path");
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

    const url = new URL(request.url);
    if (
      url.pathname.includes("/rpc") ||
      url.pathname.includes("/quests") ||
      url.pathname.includes("/graphql")
    ) {
      const PORT = getPort(url.pathname);

      const newRequest = new Request(
        `${env.SERVER_URI}:${PORT}${url.pathname.replace("/rpc", "").replace("/quests", "")}${url.search}`,
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

    const [questsStatus, graphqlStatus, rpcStatus] = await Promise.all([
      fetch(`${env.SERVER_URI}:8081/check_username/none`),
      fetch(`${env.SERVER_URI}:8080`),
      fetch(`${env.SERVER_URI}:26657`),
    ]);

    return new Response(
      JSON.stringify({
        health: {
          quests: questsStatus.ok ? "up" : "down",
          graphql: graphqlStatus.ok ? "up" : "down",
          rpc: rpcStatus.ok ? "up" : "down",
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
