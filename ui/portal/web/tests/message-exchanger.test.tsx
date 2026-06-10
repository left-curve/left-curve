import { cleanup, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { deserializeJson, serializeJson } from "@left-curve/encoding";

import { MessageExchanger } from "../../../store/src/messageExchanger";

const ecdhMocks = vi.hoisted(() => ({
  decrypt: vi.fn(),
  deriveSecret: vi.fn(),
  encrypt: vi.fn(),
  exportKey: vi.fn(),
  generateKeys: vi.fn(),
  importPublicKey: vi.fn(),
}));

vi.mock("../../../store/src/ecdh.js", () => ({
  WebCryptoECDH: {
    decrypt: ecdhMocks.decrypt,
    deriveSecret: ecdhMocks.deriveSecret,
    encrypt: ecdhMocks.encrypt,
    exportKey: ecdhMocks.exportKey,
    generateKeys: ecdhMocks.generateKeys,
    importPublicKey: ecdhMocks.importPublicKey,
  },
}));

type SocketMessage = {
  message: string;
  to: string;
  type: string;
};

type MessageData<T = unknown> = {
  id: string;
  data: T;
};

const localPublicKey = { kty: "EC", x: "local-x", y: "local-y" };
const peerPublicKey = { kty: "EC", x: "peer-x", y: "peer-y" };
const importedPeerPublicKey = { id: "imported-peer-public-key" };
const sharedSecret = { id: "shared-secret" };
const keyPair = {
  privateKey: { id: "local-private-key" },
  publicKey: { id: "local-public-key" },
};

const sockets: FakeWebSocket[] = [];

class FakeWebSocket {
  static readonly OPEN = 1;
  readonly send = vi.fn((message: string) => this.sent.push(message));
  readonly close = vi.fn(() => {
    this.readyState = 3;
  });
  readonly sent: string[] = [];
  onerror: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  readyState = FakeWebSocket.OPEN;

  constructor(readonly url: string) {
    sockets.push(this);
  }

  receive(data: string | Record<string, unknown>) {
    this.onmessage?.(
      new MessageEvent("message", {
        data: typeof data === "string" ? data : JSON.stringify(data),
      }),
    );
  }
}

function parseSocketMessage(socket: FakeWebSocket, index: number) {
  return JSON.parse(socket.sent[index]) as SocketMessage;
}

function parseSerializedMessage<T = unknown>(message: string) {
  return deserializeJson<MessageData<T>>(message);
}

async function createExchanger() {
  const create = MessageExchanger.create("wss://relay.example");
  const socket = sockets[0];

  await waitFor(() => expect(socket.onmessage).toBeTypeOf("function"));
  socket.receive("socket-a");

  return {
    exchanger: await create,
    socket,
  };
}

async function connectPeer(exchanger: MessageExchanger, socket: FakeWebSocket) {
  const connect = exchanger.createPeerConnection("socket-b");

  await waitFor(() => expect(socket.sent).toHaveLength(1));
  const handshakeInit = parseSocketMessage(socket, 0);
  const handshakeInitMessage = parseSerializedMessage<{ publicKey: JsonWebKey }>(
    handshakeInit.message,
  );

  socket.receive({
    from: "socket-b",
    message: serializeJson({
      id: handshakeInitMessage.id,
      data: {
        publicKey: peerPublicKey,
      },
    }),
    type: "handshake-ack",
  });
  await connect;

  return {
    handshakeInit,
    handshakeInitMessage,
  };
}

describe("MessageExchanger", () => {
  beforeEach(() => {
    sockets.length = 0;
    vi.stubGlobal("WebSocket", FakeWebSocket);
    ecdhMocks.generateKeys.mockResolvedValue(keyPair);
    ecdhMocks.exportKey.mockResolvedValue(localPublicKey);
    ecdhMocks.importPublicKey.mockResolvedValue(importedPeerPublicKey);
    ecdhMocks.deriveSecret.mockResolvedValue(sharedSecret);
    ecdhMocks.encrypt.mockImplementation(
      async (_secret: unknown, message: string) => `encrypted:${message}`,
    );
    ecdhMocks.decrypt.mockImplementation(async (_secret: unknown, message: string) =>
      message.replace("encrypted:", ""),
    );
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("creates a socket, completes the ECDH handshake, and exchanges encrypted messages", async () => {
    const { exchanger, socket } = await createExchanger();

    expect(socket.url).toBe("wss://relay.example");
    expect(exchanger.getSocketId()).toBe("socket-a");
    expect(ecdhMocks.generateKeys).toHaveBeenCalledOnce();

    const { handshakeInit, handshakeInitMessage } = await connectPeer(exchanger, socket);

    expect(handshakeInit).toMatchObject({
      to: "socket-b",
      type: "handshake-init",
    });
    expect(handshakeInitMessage.data).toEqual({
      publicKey: localPublicKey,
    });
    expect(ecdhMocks.importPublicKey).toHaveBeenCalledWith(peerPublicKey);
    expect(ecdhMocks.deriveSecret).toHaveBeenCalledWith(keyPair.privateKey, importedPeerPublicKey);

    const listener = vi.fn();
    const unsubscribe = exchanger.subscribe(listener);
    const response = exchanger.sendMessage<{ accepted: boolean }>({
      id: "request-1",
      message: {
        expireAt: 1_760_000_000,
        publicKey: "AQIDBA==",
      },
      type: "create-session",
    });

    await waitFor(() => expect(socket.sent).toHaveLength(2));
    const outbound = parseSocketMessage(socket, 1);
    const outboundMessage = parseSerializedMessage<string>(outbound.message);

    expect(outbound).toMatchObject({
      to: "socket-b",
      type: "create-session",
    });
    expect(outboundMessage).toEqual({
      id: "request-1",
      data: `encrypted:${serializeJson({
        expireAt: 1_760_000_000,
        publicKey: "AQIDBA==",
      })}`,
    });
    expect(ecdhMocks.encrypt).toHaveBeenCalledWith(
      sharedSecret,
      serializeJson({
        expireAt: 1_760_000_000,
        publicKey: "AQIDBA==",
      }),
    );

    socket.receive({
      from: "socket-b",
      message: serializeJson({
        id: "request-1",
        data: `encrypted:${serializeJson({ accepted: true })}`,
      }),
      type: "create-session",
    });

    await expect(response).resolves.toEqual({ accepted: true });
    expect(listener).toHaveBeenCalledWith({
      id: "request-1",
      message: {
        accepted: true,
      },
      type: "create-session",
    });

    unsubscribe();
    socket.receive({
      from: "socket-b",
      message: serializeJson({
        id: "event-1",
        data: `encrypted:${serializeJson({ status: "ready" })}`,
      }),
      type: "status",
    });

    expect(listener).toHaveBeenCalledTimes(1);

    exchanger.close();
    expect(socket.close).toHaveBeenCalledOnce();
  });

  it("acks inbound handshakes and rejects outbound messages when the socket is closed", async () => {
    const { exchanger, socket } = await createExchanger();

    socket.receive({
      from: "socket-b",
      message: serializeJson({
        id: "handshake-2",
        data: {
          publicKey: peerPublicKey,
        },
      }),
      type: "handshake-init",
    });

    await waitFor(() => expect(socket.sent).toHaveLength(1));
    const handshakeAck = parseSocketMessage(socket, 0);
    const handshakeAckMessage = parseSerializedMessage<{ publicKey: JsonWebKey }>(
      handshakeAck.message,
    );

    expect(handshakeAck).toMatchObject({
      to: "socket-b",
      type: "handshake-ack",
    });
    expect(handshakeAckMessage).toEqual({
      id: "handshake-2",
      data: {
        publicKey: localPublicKey,
      },
    });

    socket.close();

    await expect(exchanger.createPeerConnection("socket-c")).rejects.toThrow(
      "WebSocket is not open",
    );
  });

  it("delivers unsolicited encrypted peer events to active subscribers only", async () => {
    const { exchanger, socket } = await createExchanger();
    await connectPeer(exchanger, socket);
    const firstListener = vi.fn();
    const secondListener = vi.fn();
    const unsubscribeFirst = exchanger.subscribe(firstListener);
    exchanger.subscribe(secondListener);

    socket.receive({
      from: "socket-b",
      message: serializeJson({
        id: "event-1",
        data: `encrypted:${serializeJson({ status: "ready" })}`,
      }),
      type: "desktop-status",
    });

    await waitFor(() => {
      expect(firstListener).toHaveBeenCalledWith({
        id: "event-1",
        message: {
          status: "ready",
        },
        type: "desktop-status",
      });
    });
    expect(secondListener).toHaveBeenCalledWith({
      id: "event-1",
      message: {
        status: "ready",
      },
      type: "desktop-status",
    });
    expect(ecdhMocks.decrypt).toHaveBeenCalledWith(
      sharedSecret,
      `encrypted:${serializeJson({ status: "ready" })}`,
    );

    unsubscribeFirst();

    socket.receive({
      from: "socket-b",
      message: serializeJson({
        id: "event-2",
        data: `encrypted:${serializeJson({ status: "confirmed" })}`,
      }),
      type: "desktop-status",
    });

    await waitFor(() => {
      expect(secondListener).toHaveBeenCalledWith({
        id: "event-2",
        message: {
          status: "confirmed",
        },
        type: "desktop-status",
      });
    });
    expect(firstListener).toHaveBeenCalledTimes(1);
  });
});
