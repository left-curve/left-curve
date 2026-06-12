import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createStorage } from "../../../store/src/storages/createStorage";
import { useFavApplets } from "../../../store/src/hooks/useFavApplets";
import { useFavPairs } from "../../../store/src/hooks/useFavPairs";
import { useStorage } from "../../../store/src/hooks/useStorage";

type PersistedValue<T> = {
  state: {
    value: T;
    version: number;
    _hasHydrated: boolean;
  };
  version: number;
};

const hookMocks = vi.hoisted(() => ({
  useConfig: vi.fn(),
}));

vi.mock("../../../store/src/hooks/useConfig.js", () => ({
  useConfig: hookMocks.useConfig,
}));

function createRawStorage({ subscribable = true }: { subscribable?: boolean } = {}) {
  const values = new Map<string, string>();
  const listeners = new Map<string, Set<(value: string | null) => void>>();

  function notify(key: string, value: string | null) {
    for (const listener of listeners.get(key) ?? []) listener(value);
  }

  const storage = {
    getItem: vi.fn((key: string) => values.get(key) ?? null),
    removeItem: vi.fn((key: string) => {
      values.delete(key);
      notify(key, null);
    }),
    setItem: vi.fn((key: string, value: string) => {
      values.set(key, value);
      notify(key, value);
    }),
    listenerCount: (key: string) => listeners.get(key)?.size ?? 0,
  };

  if (!subscribable) return storage;

  return {
    ...storage,
    subscribe: vi.fn((key: string, listener: (value: string | null) => void) => {
      const current = listeners.get(key) ?? new Set<(value: string | null) => void>();
      current.add(listener);
      listeners.set(key, current);
      return () => {
        current.delete(listener);
        if (current.size === 0) listeners.delete(key);
      };
    }),
  };
}

let keyIndex = 0;

function nextKey(label: string) {
  keyIndex += 1;
  return `unit.${label}.${keyIndex}`;
}

function configureStorage(prefix = nextKey("prefix"), options: { subscribable?: boolean } = {}) {
  const rawStorage = createRawStorage(options);
  const storage = createStorage({
    key: prefix,
    storage: rawStorage,
  });

  hookMocks.useConfig.mockReturnValue({ storage });

  return { rawStorage, storage };
}

describe("storage hooks", () => {
  beforeEach(() => {
    configureStorage();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("hydrates migrated persisted values and writes updater results back to storage", async () => {
    const { storage } = configureStorage();
    const key = nextKey("migrated");
    storage.setItem(key, {
      state: {
        _hasHydrated: false,
        value: ["legacy"],
        version: 0,
      },
      version: 0,
    } satisfies PersistedValue<string[]>);

    const { result } = renderHook(() =>
      useStorage<string[]>(key, {
        initialValue: ["initial"],
        migrations: {
          "*": () => ["migrated"],
        },
        version: 2,
      }),
    );

    await waitFor(() => expect(result.current[2]).toBe(true));
    await waitFor(() => expect(result.current[0]).toEqual(["migrated"]));

    act(() => {
      result.current[1]((previous) => [...previous, "saved"]);
    });

    await waitFor(() => expect(result.current[0]).toEqual(["migrated", "saved"]));
    expect(storage.getItem(key)).toMatchObject({
      state: {
        value: ["migrated", "saved"],
      },
      version: 2,
    });
  });

  it("hydrates version-specific storage migrations when no wildcard migration is configured", async () => {
    const { storage } = configureStorage();
    const key = nextKey("version-migrated");
    storage.setItem(key, {
      state: {
        _hasHydrated: false,
        value: ["legacy"],
        version: 1,
      },
      version: 1,
    } satisfies PersistedValue<string[]>);

    const { result } = renderHook(() =>
      useStorage<string[]>(key, {
        initialValue: ["initial"],
        migrations: {
          1: (value: string[]) => ["v1", ...value],
        },
        version: 2,
      }),
    );

    await waitFor(() => expect(result.current[2]).toBe(true));
    await waitFor(() => expect(result.current[0]).toEqual(["v1", "legacy"]));

    expect(storage.getItem(key)).toMatchObject({
      state: {
        value: ["v1", "legacy"],
      },
      version: 2,
    });
  });

  it("returns the initial value while disabled without reading persisted state into the hook", async () => {
    const { storage } = configureStorage();
    const key = nextKey("disabled");
    storage.setItem(key, {
      state: {
        _hasHydrated: false,
        value: "stored",
        version: 1,
      },
      version: 1,
    } satisfies PersistedValue<string>);

    const { result } = renderHook(() =>
      useStorage(key, {
        enabled: false,
        initialValue: "initial",
      }),
    );

    await waitFor(() => expect(result.current[2]).toBe(false));
    expect(result.current[0]).toBe("initial");
  });

  it("shares storage state between multiple consumers with the same key", async () => {
    const { storage } = configureStorage();
    const key = nextKey("shared-consumers");
    const first = renderHook(() =>
      useStorage<string[]>(key, {
        initialValue: [],
      }),
    );
    const second = renderHook(() =>
      useStorage<string[]>(key, {
        initialValue: [],
      }),
    );

    await waitFor(() => expect(first.result.current[2]).toBe(true));
    await waitFor(() => expect(second.result.current[2]).toBe(true));

    act(() => {
      first.result.current[1]((previous) => [...previous, "favorite"]);
    });

    await waitFor(() => expect(first.result.current[0]).toEqual(["favorite"]));
    await waitFor(() => expect(second.result.current[0]).toEqual(["favorite"]));
    expect(storage.getItem(key)).toMatchObject({
      state: {
        value: ["favorite"],
      },
    });
  });

  it("syncs storage updates through adapter subscriptions when requested", async () => {
    const { storage } = configureStorage();
    const key = nextKey("sync");
    const { result } = renderHook(() =>
      useStorage<string[]>(key, {
        initialValue: [],
        sync: true,
      }),
    );

    await waitFor(() => expect(result.current[2]).toBe(true));

    act(() => {
      storage.setItem(key, {
        state: {
          value: ["remote"],
          version: 1,
        },
        version: 1,
      } satisfies PersistedValue<string[]>);
    });

    await waitFor(() => expect(result.current[0]).toEqual(["remote"]));

    act(() => {
      result.current[1]((previous) => [...previous, "local"]);
    });

    await waitFor(() => expect(result.current[0]).toEqual(["remote", "local"]));
    expect(storage.getItem(key)).toMatchObject({
      state: {
        value: ["remote", "local"],
      },
    });
  });

  it("cleans up adapter subscription listeners for synced storage on unmount", async () => {
    const { rawStorage, storage } = configureStorage();
    const key = nextKey("sync-cleanup");
    const { unmount } = renderHook(() =>
      useStorage<string[]>(key, {
        initialValue: [],
        sync: true,
      }),
    );

    await waitFor(() => expect(rawStorage.listenerCount(`${storage.key}.${key}`)).toBe(1));

    unmount();

    expect(rawStorage.listenerCount(`${storage.key}.${key}`)).toBe(0);
  });

  it("keeps synced storage functional when the adapter has no subscription support", async () => {
    const { storage } = configureStorage(nextKey("no-subscribe-prefix"), {
      subscribable: false,
    });
    const key = nextKey("sync-without-subscribe");

    const { result } = renderHook(() =>
      useStorage<string[]>(key, {
        initialValue: [],
        sync: true,
      }),
    );

    await waitFor(() => expect(result.current[2]).toBe(true));

    act(() => {
      result.current[1]((previous) => [...previous, "local"]);
    });

    await waitFor(() => expect(result.current[0]).toEqual(["local"]));
    expect(storage.getItem(key)).toMatchObject({
      state: {
        value: ["local"],
      },
    });
  });

  it("migrates favorite pair storage from dashed tickers to compact tickers", async () => {
    const { storage } = configureStorage();
    storage.setItem("favorites.pairs", {
      state: {
        _hasHydrated: false,
        value: ["BTC-USD", "ETH-USD", "BTCUSD"],
        version: 0,
      },
      version: 0,
    } satisfies PersistedValue<string[]>);

    const { result } = renderHook(() => useFavPairs());

    await waitFor(() => expect(result.current.favPairs).toEqual(["BTCUSD", "ETHUSD"]));
    expect(storage.getItem("favorites.pairs")).toMatchObject({
      state: {
        value: ["BTCUSD", "ETHUSD"],
      },
      version: 1,
    });
  });

  it("adds, removes, and toggles favorite pairs without duplicating entries", async () => {
    configureStorage();
    const { result } = renderHook(() => useFavPairs());

    await waitFor(() => expect(result.current.favPairs).toEqual([]));

    act(() => {
      result.current.addFavPair("BTCUSD");
      result.current.addFavPair("BTCUSD");
    });

    await waitFor(() => expect(result.current.favPairs).toEqual(["BTCUSD"]));
    expect(result.current.hasFavPair("BTCUSD")).toBe(true);

    act(() => {
      result.current.toggleFavPair("BTCUSD");
    });

    await waitFor(() => expect(result.current.favPairs).toEqual([]));

    act(() => {
      result.current.toggleFavPair("ETHUSD");
      result.current.removeFavPair("missing");
    });

    await waitFor(() => expect(result.current.favPairs).toEqual(["ETHUSD"]));
  });

  it("uses the migrated default applet list and supports applet preference edits", async () => {
    configureStorage();
    const { result } = renderHook(() => useFavApplets());

    await waitFor(() =>
      expect(result.current.favApplets).toEqual([
        "earn",
        "trade",
        "bridge",
        "transfer",
        "create-account",
        "settings",
        "referral",
      ]),
    );

    act(() => {
      result.current.addFavApplet({ id: "portfolio", title: "Portfolio" });
      result.current.removeFavApplet({ id: "bridge", title: "Bridge" });
    });

    await waitFor(() =>
      expect(result.current.favApplets).toEqual([
        "earn",
        "trade",
        "transfer",
        "create-account",
        "settings",
        "referral",
        "portfolio",
      ]),
    );
  });
});
