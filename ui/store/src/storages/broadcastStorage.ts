import type { AbstractStorage } from "../types/storage.js";

type BroadcastMessage =
  | { key: string; source: string; type: "request" }
  | { key: string; source: string; type: "set"; value: string | null };

export type CreateBroadcastStorageParameters = {
  channelName?: string;
  hydrate?: boolean;
  mirror?: boolean;
  storage: AbstractStorage;
};

function createChannel(name: string): BroadcastChannel | null {
  try {
    return new BroadcastChannel(name);
  } catch {
    return null;
  }
}

export function createBroadcastStorage(
  parameters: CreateBroadcastStorageParameters,
): AbstractStorage {
  const { channelName = "dango.storage", hydrate = false, mirror = false, storage } = parameters;

  const channel = createChannel(channelName);
  const source = Math.random().toString(36).slice(2);
  const listeners = new Map<string, Set<(value: string | null) => void>>();

  function notify(key: string, value: string | null) {
    for (const listener of listeners.get(key) ?? []) listener(value);
  }

  async function getRawValue(key: string): Promise<string | null> {
    const value = await storage.getItem(key);
    return value ?? null;
  }

  function publish(key: string, value: string | null) {
    channel?.postMessage({ key, source, type: "set", value } satisfies BroadcastMessage);
  }

  async function handleMessage(data: BroadcastMessage) {
    if (!data || data.source === source) return;

    if (data.type === "request") {
      const value = await getRawValue(data.key);
      if (value !== null) publish(data.key, value);
      return;
    }

    if (mirror) {
      if (data.value === null) await storage.removeItem(data.key);
      else await storage.setItem(data.key, data.value);
    }

    notify(data.key, data.value);
  }

  channel?.addEventListener("message", ({ data }: MessageEvent<BroadcastMessage>) => {
    void handleMessage(data).catch(() => undefined);
  });

  return {
    ...storage,
    getItem(key) {
      return storage.getItem(key);
    },
    removeItem(key) {
      const result = storage.removeItem(key);
      void Promise.resolve(result)
        .then(() => {
          publish(key, null);
          notify(key, null);
        })
        .catch(() => undefined);
      return result;
    },
    setItem(key, value) {
      const result = storage.setItem(key, value);
      void Promise.resolve(result)
        .then(() => {
          publish(key, value);
          notify(key, value);
        })
        .catch(() => undefined);
      return result;
    },
    subscribe(key, listener) {
      const current = listeners.get(key) ?? new Set<(value: string | null) => void>();
      current.add(listener);
      listeners.set(key, current);

      if (hydrate) {
        channel?.postMessage({ key, source, type: "request" } satisfies BroadcastMessage);
      }

      return () => {
        current.delete(listener);
        if (current.size === 0) listeners.delete(key);
      };
    },
  };
}
