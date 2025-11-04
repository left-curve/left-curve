import { uid, withResolvers } from "@left-curve/dango/utils";
import { WebCryptoECDH } from "./ecdh.js";
import { deserializeJson, serializeJson } from "@left-curve/dango/encoding";

type MessageData<T = any> = {
  id: string;
  data: T;
};

export class MessageExchanger {
  #ws: WebSocket;
  #socketId: string;
  #peerSocketId: string | null = null;
  #resolver: Map<string, { resolve: (value: unknown) => void; reject: (reason?: unknown) => void }>;
  #listeners: Set<(message: { id: string; type: string; message: unknown }) => void>;
  #keyPair: CryptoKeyPair;
  #sharedSecret: CryptoKey | null = null;
  static async create(url: string) {
    const ws = new WebSocket(url);
    const keyPair = await WebCryptoECDH.generateKeys();
    const { promise, resolve } = withResolvers<string>();
    ws.onmessage = (e) => resolve(e.data);
    const socketId = await promise;
    return new MessageExchanger(ws, socketId, keyPair);
  }

  protected constructor(ws: WebSocket, socketId: string, keyPair: CryptoKeyPair) {
    this.#ws = ws;
    this.#socketId = socketId;
    this.#keyPair = keyPair;
    this.#listeners = new Set();
    this.#resolver = new Map();
    this.#ws.onclose = this.#onWebSocketClose.bind(this);
    this.#ws.onerror = this.#onWebSocketError.bind(this);
    this.#ws.onmessage = this.#onWebSocketMessage.bind(this);
  }

  #webSocketSend(data: { to: string; type: string; message: MessageData }) {
    if (this.#ws.readyState !== WebSocket.OPEN) {
      throw new Error("WebSocket is not open");
    }

    this.#ws.send(
      JSON.stringify({
        to: data.to,
        type: data.type,
        message: serializeJson(data.message),
      }),
    );
  }

  #onWebSocketClose() {
    console.log("WebSocket Disconnected");
  }

  #onWebSocketError(error: unknown) {
    console.error("websocket error: ", error);
  }

  async #onWebSocketMessage(event: MessageEvent) {
    const { type, message, from } = JSON.parse(event.data);

    if (type === "handshake-init") return this.#onHandShakeInit(from, message);
    if (type === "handshake-ack") return this.#onHandShakeAck(message);

    const { id, data } = deserializeJson<MessageData>(message);

    const resolver = this.#resolver.get(id);

    const uncryptedData = await WebCryptoECDH.decrypt(this.#sharedSecret!, data);

    const response = { id, type, message: deserializeJson(uncryptedData) };

    if (resolver) {
      resolver.resolve(response.message);
      this.#resolver.delete(id);
    }

    this.#listeners.forEach((listener) => listener(response));
  }

  async #onHandShakeInit(peerSocketId: string, message: string) {
    this.#peerSocketId = peerSocketId;
    const { id, data } = deserializeJson<MessageData<{ publicKey: JsonWebKey }>>(message);
    console.log(data);
    const peerPublicKey = await WebCryptoECDH.importPublicKey(data.publicKey);
    this.#sharedSecret = await WebCryptoECDH.deriveSecret(this.#keyPair.privateKey, peerPublicKey);

    const publicKey = await WebCryptoECDH.exportKey(this.#keyPair.publicKey);

    this.#webSocketSend({
      to: this.#peerSocketId,
      message: { id, data: { publicKey } },
      type: "handshake-ack",
    });
  }

  async #onHandShakeAck(message: string) {
    const { id, data } = deserializeJson<MessageData<{ publicKey: JsonWebKey }>>(message);
    const peerPublicKey = await WebCryptoECDH.importPublicKey(data.publicKey);
    this.#sharedSecret = await WebCryptoECDH.deriveSecret(this.#keyPair.privateKey, peerPublicKey);

    const resolver = this.#resolver.get(id);
    resolver?.resolve(this.#sharedSecret);
  }

  async createPeerConnection(socketId: string): Promise<void> {
    this.#peerSocketId = socketId;
    const id = uid();

    const publicKey = await WebCryptoECDH.exportKey(this.#keyPair.publicKey);

    this.#webSocketSend({
      to: this.#peerSocketId,
      message: {
        id,
        data: { publicKey },
      },
      type: "handshake-init",
    });

    const { promise, resolve, reject } = withResolvers();
    this.#resolver.set(id, { resolve, reject });
    await promise;
  }

  async sendMessage<R = unknown>(m: { id?: string; type?: string; message: unknown }): Promise<R> {
    const { id = uid(), type = "message", message } = m;

    const { promise, resolve, reject } = withResolvers();
    this.#resolver.set(id, { resolve, reject });

    const encryptedMessage = await WebCryptoECDH.encrypt(
      this.#sharedSecret!,
      serializeJson(message),
    );

    this.#webSocketSend({ to: this.#peerSocketId!, type, message: { id, data: encryptedMessage } });
    return promise as Promise<R>;
  }

  subscribe(listener: <T = any>(message: { id: string; type: string; message: T }) => void) {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  getSocketId() {
    return this.#socketId;
  }

  close() {
    this.#ws.close();
  }
}
