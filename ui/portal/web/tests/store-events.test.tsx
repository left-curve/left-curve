import { describe, expect, it, vi } from "vitest";

import { createEmitter } from "../../../store/src/createEmitter";
import { createEventBus } from "../../../store/src/createEventBus";

type BusEvents = {
  account: {
    address: string;
  };
  status: {
    value: string;
  };
};

type ConnectorEvents = {
  change: {
    chainId: string;
    keyHash: string;
  };
  disconnect: never;
};

describe("store event primitives", () => {
  it("publishes payloads to every subscriber for the matching event", () => {
    const bus = createEventBus<BusEvents>();
    const accountListener = vi.fn();
    const secondAccountListener = vi.fn();
    const statusListener = vi.fn();

    bus.subscribe("account", accountListener);
    bus.subscribe("account", secondAccountListener);
    bus.subscribe("status", statusListener);

    bus.publish("account", {
      address: "0x6163636f756e742d6576656e742d3000000000",
    });

    expect(accountListener).toHaveBeenCalledWith({
      address: "0x6163636f756e742d6576656e742d3000000000",
    });
    expect(secondAccountListener).toHaveBeenCalledWith({
      address: "0x6163636f756e742d6576656e742d3000000000",
    });
    expect(statusListener).not.toHaveBeenCalled();
  });

  it("unsubscribes only the matching event bus callback", () => {
    const bus = createEventBus<BusEvents>();
    const first = vi.fn();
    const second = vi.fn();
    const unsubscribeFirst = bus.subscribe("account", first);

    bus.subscribe("account", second);
    unsubscribeFirst();
    bus.publish("account", {
      address: "0x756e737562736372696265642d300000000000",
    });

    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledWith({
      address: "0x756e737562736372696265642d300000000000",
    });
  });

  it("allows publishing events before listeners are registered", () => {
    const bus = createEventBus<BusEvents>();

    expect(() =>
      bus.publish("account", {
        address: "0x6e6f2d6c697374656e6572732d300000000000",
      }),
    ).not.toThrow();
  });

  it("injects emitter uid into payload events and tracks listener counts", () => {
    const emitter = createEmitter<ConnectorEvents>("connector-uid");
    const listener = vi.fn();

    expect(emitter.listenerCount("change")).toBe(0);

    emitter.on("change", listener);
    expect(emitter.listenerCount("change")).toBe(1);

    emitter.emit("change", {
      chainId: "dango-dev-1",
      keyHash: "0x656d69747465722d6b657900000000000000000000000000000000000000",
    });

    expect(listener).toHaveBeenCalledWith({
      chainId: "dango-dev-1",
      keyHash: "0x656d69747465722d6b657900000000000000000000000000000000000000",
      uid: "connector-uid",
    });

    emitter.off("change", listener);
    expect(emitter.listenerCount("change")).toBe(0);
  });

  it("runs once listeners exactly one time", () => {
    const emitter = createEmitter<ConnectorEvents>("connector-uid");
    const listener = vi.fn();

    emitter.once("change", listener);
    emitter.emit("change", {
      chainId: "dango-dev-1",
      keyHash: "0x6f6e63652d6c697374656e65722d3000000000000000000000000000000000",
    });
    emitter.emit("change", {
      chainId: "dango-dev-1",
      keyHash: "0x6f6e63652d6c697374656e65722d3100000000000000000000000000000000",
    });

    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenCalledWith({
      chainId: "dango-dev-1",
      keyHash: "0x6f6e63652d6c697374656e65722d3000000000000000000000000000000000",
      uid: "connector-uid",
    });
    expect(emitter.listenerCount("change")).toBe(0);
  });

  it("emits uid-only payloads for no-data connector events", () => {
    const emitter = createEmitter<ConnectorEvents>("connector-uid");
    const listener = vi.fn();

    emitter.on("disconnect", listener);
    emitter.emit("disconnect");

    expect(listener).toHaveBeenCalledWith({
      uid: "connector-uid",
    });
  });
});
