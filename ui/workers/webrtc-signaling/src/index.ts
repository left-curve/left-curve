import { DurableObject } from "cloudflare:workers";

type Env = {
  WEBSOCKET: DurableObjectNamespace<SignalingServer>;
};

export class SignalingServer extends DurableObject<Env> {
  socket: WebSocket;
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    const response = this.ctx.getWebSockets();
    const [socket] = response;
    this.socket = socket;
  }

  verify() {
    return !!this.socket;
  }

  fetch(req: Request) {
    const websocketPair = new WebSocketPair();
    const [client, server] = Object.values(websocketPair);

    this.ctx.acceptWebSocket(server);
    this.socket = server;
    server.send(this.ctx.id.toString());

    return new Response(null, {
      status: 101,
      webSocket: client,
    });
  }

  webSocketMessage(ws: WebSocket, payload: string) {
    const { type, to, message } = JSON.parse(payload);
    const socket = this.env.WEBSOCKET.get(this.env.WEBSOCKET.idFromString(to));
    if (!socket.verify()) return ws.send("error: websocket not found");
    socket.webSocketSend(JSON.stringify({ type, from: this.ctx.id.toString(), message }));
  }

  webSocketError(ws: WebSocket, error: string) {
    ws.close(1011, "Unexpected error");
  }

  webSocketClose(ws: WebSocket, code: number, reason: string): void {
    ws.close(code, reason);
  }

  webSocketSend(message: string) {
    this.socket.send(message);
  }
}

export default {
  async fetch(request: Request, env: Env) {
    if (request.headers.get("Upgrade") === "websocket") {
      const ws = env.WEBSOCKET.get(env.WEBSOCKET.newUniqueId());
      return ws.fetch(request);
    }

    return new Response(JSON.stringify({ health: "up" }));
  },
};
