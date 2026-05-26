import type { EventEmitter } from "eventemitter3";
import { describe, expect, it, vi } from "vitest";

import { createSubscription } from "./createSubscription.js";

const createEmitter = () => {
  const listeners = new Map<string, Set<(...args: unknown[]) => void>>();

  return {
    on(event: string, listener: (...args: unknown[]) => void) {
      const eventListeners = listeners.get(event) ?? new Set();
      eventListeners.add(listener);
      listeners.set(event, eventListeners);
      return this;
    },
    off(event: string, listener: (...args: unknown[]) => void) {
      listeners.get(event)?.delete(listener);
      return this;
    },
    emit(event: string, ...args: unknown[]) {
      for (const listener of listeners.get(event) ?? []) listener(...args);
      return true;
    },
  } as unknown as EventEmitter;
};

describe("createSubscription", () => {
  it("cleans up the previous ws subscription before resubscribing", () => {
    const emitter = createEmitter();
    const firstUnsubscribe = vi.fn();
    const secondUnsubscribe = vi.fn();
    const wsSubscribe = vi.fn().mockReturnValueOnce(firstUnsubscribe).mockReturnValueOnce(secondUnsubscribe);

    const unsubscribe = createSubscription(
      {
        wsSubscribe,
        httpInterval: 1_000,
        emitter,
        getStatus: () => ({ isConnected: true }),
      },
      vi.fn(),
    );

    emitter.emit("connected");

    expect(firstUnsubscribe).toHaveBeenCalledTimes(1);
    expect(secondUnsubscribe).not.toHaveBeenCalled();
    expect(wsSubscribe).toHaveBeenCalledTimes(2);

    unsubscribe();

    expect(secondUnsubscribe).toHaveBeenCalledTimes(1);
  });
});
