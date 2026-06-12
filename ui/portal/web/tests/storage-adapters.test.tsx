import { describe, expect, it, vi } from "vitest";

import { createAsyncStorage, createStorage } from "../../../store/src/storages/createStorage";
import { createMemoryStorage } from "../../../store/src/storages/memoryStorage";

function createRawStorage() {
  const values = new Map<string, string>();

  return {
    getItem: vi.fn((key: string) => values.get(key) ?? null),
    removeItem: vi.fn((key: string) => {
      values.delete(key);
    }),
    setItem: vi.fn((key: string, value: string) => {
      values.set(key, value);
    }),
  };
}

describe("storage adapters", () => {
  it("prefixes keys and round-trips serialized values through sync storage", () => {
    const rawStorage = createRawStorage();
    const storage = createStorage<{ account: { address: string } }>({
      key: "portal",
      storage: rawStorage,
    });

    storage.setItem("account", {
      address: "0x73746f726167652d6163636f756e742d300000",
    });

    expect(rawStorage.setItem).toHaveBeenCalledWith(
      "portal.account",
      '{"json":{"address":"0x73746f726167652d6163636f756e742d300000"}}',
    );
    expect(storage.getItem("account")).toEqual({
      address: "0x73746f726167652d6163636f756e742d300000",
    });
    expect(rawStorage.getItem).toHaveBeenCalledWith("portal.account");
  });

  it("returns defaults for missing values and removes prefixed keys for null writes", () => {
    const rawStorage = createRawStorage();
    const storage = createStorage<{ theme: string }>({
      key: "portal",
      storage: rawStorage,
    });

    expect(storage.getItem("theme", "dark")).toBe("dark");

    storage.setItem("theme", "light");
    storage.setItem("theme", null);

    expect(rawStorage.removeItem).toHaveBeenCalledWith("portal.theme");
    expect(storage.getItem("theme")).toBeNull();
  });

  it("supports custom serialization boundaries", () => {
    const rawStorage = createRawStorage();
    const storage = createStorage<{ count: number }>({
      deserialize: (value) => Number(value.replace("count:", "")),
      key: "custom",
      serialize: (value) => `count:${value}`,
      storage: rawStorage,
    });

    storage.setItem("count", 7);

    expect(rawStorage.setItem).toHaveBeenCalledWith("custom.count", "count:7");
    expect(storage.getItem("count")).toBe(7);
  });

  it("keeps memory storage isolated by key and removes stored entries", () => {
    const storage = createMemoryStorage();

    storage.setItem("first", "one");
    storage.setItem("second", "two");
    storage.removeItem("first");

    expect(storage.getItem("first")).toBeNull();
    expect(storage.getItem("second")).toBe("two");
  });

  it("awaits async storage operations and falls back to defaults when reads reject", async () => {
    const rawStorage = {
      getItem: vi.fn((key: string) => {
        if (key === "async.rejected") return Promise.reject(new Error("storage unavailable"));
        return Promise.resolve(key === "async.saved" ? '{"json":{"ready":true}}' : null);
      }),
      removeItem: vi.fn(() => Promise.resolve()),
      setItem: vi.fn(() => Promise.resolve()),
    };
    const storage = createAsyncStorage<{ saved: { ready: boolean }; rejected: string }>({
      key: "async",
      storage: rawStorage,
    });

    await expect(storage.getItem("saved")).resolves.toEqual({
      ready: true,
    });
    await expect(storage.getItem("rejected", "fallback")).resolves.toBe("fallback");

    await storage.setItem("saved", {
      ready: false,
    });
    await storage.setItem("saved", null);

    expect(rawStorage.setItem).toHaveBeenCalledWith("async.saved", '{"json":{"ready":false}}');
    expect(rawStorage.removeItem).toHaveBeenCalledWith("async.saved");
  });

  it("does not throw when async storage writes or removals reject", async () => {
    const writeError = new Error("quota exceeded");
    const removeError = new Error("remove unavailable");
    const rawStorage = {
      getItem: vi.fn(() => Promise.resolve(null)),
      removeItem: vi.fn(() => Promise.reject(removeError)),
      setItem: vi.fn(() => Promise.reject(writeError)),
    };
    const storage = createAsyncStorage<{ settings: { theme: string } }>({
      key: "async",
      storage: rawStorage,
    });

    await expect(storage.setItem("settings", { theme: "dark" })).resolves.toBeUndefined();
    await expect(storage.setItem("settings", null)).resolves.toBeUndefined();
    await expect(storage.removeItem("settings")).resolves.toBeUndefined();

    expect(rawStorage.setItem).toHaveBeenCalledWith("async.settings", '{"json":{"theme":"dark"}}');
    expect(rawStorage.removeItem).toHaveBeenNthCalledWith(1, "async.settings");
    expect(rawStorage.removeItem).toHaveBeenNthCalledWith(2, "async.settings");
  });
});
