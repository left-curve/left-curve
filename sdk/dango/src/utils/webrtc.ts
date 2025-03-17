import type { Prettify } from "@left-curve/sdk/types";
import { withResolvers } from "@left-curve/sdk/utils";
import { deserializeJson, serializeJson } from "../encoding.js";
import type { DataChannelConfig, DataChannelMessage } from "../types/webrtrc.js";

export class DataChannel {
  #ws: WebSocket;
  #cfg: DataChannelConfig;
  #socketId: string;
  #connection: RTCPeerConnection;
  #dataChannel: RTCDataChannel;
  #resolver: Map<string, { resolve: (value: unknown) => void; reject: (reason?: unknown) => void }>;
  #metadata: { promiseId: string; socketId: string } | undefined;
  #listeners: Set<(message: DataChannelMessage) => void>;
  static async create(url: string, cfg?: Partial<DataChannelConfig>) {
    const { promise, resolve } = withResolvers<string>();
    const ws = new WebSocket(url);
    ws.onmessage = (e) => resolve(e.data);
    ws.onopen = () => cfg?.logs && console.log("WebSocket Connected");
    const socketId = await promise;
    return new DataChannel(ws, socketId, cfg);
  }

  protected constructor(ws: WebSocket, socketId: string, cfg?: Partial<DataChannelConfig>) {
    this.#ws = ws;
    this.#cfg = Object.assign(this.#getDefaultConfig(), cfg);
    this.#socketId = socketId;
    this.#listeners = new Set();
    this.#resolver = new Map();
    this.#connection = new RTCPeerConnection(this.#cfg.rtcConfiguration);
    this.#dataChannel = this.#connection.createDataChannel(this.#cfg.channelName);
    // websockets bindings
    this.#ws.onclose = this.#onWebSocketClose.bind(this);
    this.#ws.onerror = this.#onWebSocketError.bind(this);
    this.#ws.onmessage = this.#onWebSocketMessage.bind(this);
    // peer connection bindings
    this.#connection.onicecandidate = this.#onIceCandidate.bind(this);
    this.#connection.ondatachannel = this.#onDataChannel.bind(this);
    // datachannel bindings
    this.#dataChannel.onopen = this.#onDataChannelOpen.bind(this);
  }

  #getDefaultConfig(): DataChannelConfig {
    return {
      logs: false,
      channelName: "left-curve-channel",
      rtcConfiguration: {
        iceServers: [
          {
            urls: ["stun:stun1.l.google.com:19302", "stun:stun2.l.google.com:19302"],
          },
        ],
        iceCandidatePoolSize: 10,
      },
    };
  }

  #webSocketSend(data: Record<string, unknown>) {
    if (this.#ws.readyState !== WebSocket.OPEN) {
      throw new Error("WebSocket is not open");
    }
    this.#ws.send(JSON.stringify(data));
  }

  #onWebSocketClose() {
    if (this.#cfg.logs) console.log("WebSocket Disconnected");
  }

  #onWebSocketError(error: unknown) {
    if (this.#cfg.logs) console.error("websocket error: ", error);
  }

  #onIceCandidate({ candidate }: RTCPeerConnectionIceEvent) {
    if (!candidate || !this.#metadata) return;
    this.#webSocketSend({
      type: "candidate",
      message: { candidate },
      to: this.#metadata.socketId,
    });
  }

  #onDataChannel({ channel }: RTCDataChannelEvent) {
    channel.onmessage = this.#onDataChannelMessage.bind(this);
  }

  #onDataChannelOpen() {
    if (this.#metadata) {
      const resolver = this.#resolver.get(this.#metadata.promiseId);
      if (!resolver) throw new Error("datachannel error: resolver not found");
      if (this.#cfg.logs) console.log("Data Channel Opened");
      resolver.resolve("Data Channel Opened");
      this.#resolver.delete(this.#metadata.promiseId);
    }
  }

  #onDataChannelMessage(event: MessageEvent) {
    const data = deserializeJson<DataChannelMessage>(event.data);
    const resolver = this.#resolver.get(data.id);

    if (resolver) {
      resolver.resolve(data.message);
      this.#resolver.delete(data.id);
      return;
    }

    this.#listeners.forEach((listener) => listener(data));
  }

  async #onOffer(from: string, offer: RTCSessionDescriptionInit) {
    await this.#connection.setRemoteDescription(offer);
    const answer = await this.#connection.createAnswer();
    await this.#connection.setLocalDescription(answer);
    this.#webSocketSend({ to: from, message: { answer }, type: "answer" });
  }

  async #onAnswer(answer: RTCSessionDescriptionInit) {
    if (this.#connection.currentRemoteDescription) return;
    await this.#connection.setRemoteDescription(answer);
  }

  async #onCandidate(candidate: RTCIceCandidate) {
    await this.#connection.addIceCandidate(new RTCIceCandidate(candidate));
  }

  #onWebSocketMessage(event: MessageEvent) {
    const { type, message, from } = JSON.parse(event.data);

    switch (type) {
      case "offer":
        return this.#onOffer(from, message.offer);
      case "answer":
        return this.#onAnswer(message.answer);
      case "candidate":
        return this.#onCandidate(message.candidate);
      default:
        throw new Error("websocket error: unknown message type");
    }
  }

  async createPeerConnection(socketId: string): Promise<void> {
    const promiseId = crypto.randomUUID();
    this.#metadata = { promiseId, socketId };
    const offer = await this.#connection.createOffer();
    await this.#connection.setLocalDescription(offer);
    this.#webSocketSend({
      to: this.#metadata.socketId,
      message: { offer },
      type: "offer",
    });
    const { promise, resolve, reject } = withResolvers();
    this.#resolver.set(promiseId, { resolve, reject });
    await promise;
  }

  async sendAsyncMessage<R = unknown>(
    m: Prettify<Omit<DataChannelMessage, "id"> & { id?: string }>,
  ): Promise<R> {
    if (this.#dataChannel.readyState !== "open") {
      throw new Error("error: data channel is not open");
    }
    const { id = crypto.randomUUID(), type, message } = m;

    this.#dataChannel.send(serializeJson({ id, type, message }));
    const { promise, resolve, reject } = withResolvers();
    this.#resolver.set(id, { resolve, reject });
    return promise as Promise<R>;
  }

  sendMessage(m: Partial<DataChannelMessage>): void {
    if (this.#dataChannel.readyState !== "open") {
      throw new Error("error: data channel is not open");
    }

    const { id = crypto.randomUUID(), type = "default", message } = m;

    this.#dataChannel.send(serializeJson({ id, type, message }));
  }

  subscribe(listener: (message: DataChannelMessage) => void) {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  getSocketId() {
    return this.#socketId;
  }

  close() {
    this.#dataChannel.close();
    this.#connection.close();
    this.close();
  }
}
