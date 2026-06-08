import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type React from "react";
import { describe, expect, it, vi } from "vitest";

import { useStorage } from "../../../store/src/hooks/useStorage";
import { createAsyncStorage, createStorage } from "../../../store/src/storages/createStorage";
import { createMemoryStorage } from "../../../store/src/storages/memoryStorage";

import type { AbstractStorage, Storage } from "../../../store/src/types/storage";

const { configStorage } = vi.hoisted(() => ({
  configStorage: { current: undefined as Storage | undefined },
}));

vi.mock("../../../store/src/hooks/useConfig", () => ({
  useConfig: () => ({ storage: configStorage.current }),
}));

vi.mock("../../../store/src/hooks/useConfig.js", () => ({
  useConfig: () => ({ storage: configStorage.current }),
}));

function renderWithStorage(children: React.ReactNode, storage: Storage) {
  configStorage.current = storage;
  return render(children);
}

function createDeferredStorage(initialValues: Record<string, string> = {}) {
  const values = new Map(Object.entries(initialValues));
  const reads: Array<(value: string | null) => void> = [];

  const storage: AbstractStorage = {
    getItem(key) {
      return new Promise<string | null>((resolve) => {
        reads.push(() => resolve(values.get(key) ?? null));
      });
    },
    setItem(key, value) {
      values.set(key, value);
    },
    removeItem(key) {
      values.delete(key);
    },
  };

  return {
    reads,
    storage,
    values,
    resolveNextRead() {
      const resolve = reads.shift();
      if (!resolve) throw new Error("no pending read");
      resolve(values.get("dango.race") ?? null);
    },
  };
}

describe("storage adapters", () => {
  it("notifies memory storage subscribers with raw values", () => {
    const storage = createMemoryStorage();
    const listener = vi.fn();

    storage.subscribe?.("key", listener);

    storage.setItem("key", "value");
    storage.removeItem("key");

    expect(listener).toHaveBeenNthCalledWith(1, "value");
    expect(listener).toHaveBeenNthCalledWith(2, null);
  });

  it("prefixes and deserializes storage subscriptions", () => {
    const storage = createStorage({ storage: createMemoryStorage() });
    const listener = vi.fn();

    storage.subscribe?.("setting", listener);
    storage.setItem("setting", { enabled: true });

    expect(listener).toHaveBeenCalledWith({ enabled: true });
  });
});

describe("useStorage", () => {
  it("shares optimistic updates across hook instances for the same storage key", async () => {
    const storage = createStorage({ storage: createMemoryStorage() });

    function Consumer({ id }: { id: string }) {
      const [value, setValue, hasHydrated] = useStorage<string>("shared", {
        initialValue: "initial",
        storage,
      });
      return (
        <button
          data-hydrated={String(hasHydrated)}
          data-testid={id}
          type="button"
          onClick={() => setValue("updated")}
        >
          {value}
        </button>
      );
    }

    renderWithStorage(
      <>
        <Consumer id="first" />
        <Consumer id="second" />
      </>,
      storage,
    );

    await waitFor(() =>
      expect(screen.getByTestId("first")).toHaveAttribute("data-hydrated", "true"),
    );

    act(() => {
      fireEvent.click(screen.getByTestId("first"));
    });

    expect(screen.getByTestId("first")).toHaveTextContent("updated");
    expect(screen.getByTestId("second")).toHaveTextContent("updated");
  });

  it("does not let stale async hydration overwrite a newer local write", async () => {
    const deferred = createDeferredStorage();
    const storage = createAsyncStorage({ storage: deferred.storage });
    await storage.setItem("race", {
      state: { value: "stored", version: 1 },
      version: 1,
    });

    function Consumer() {
      const [value, setValue, hasHydrated] = useStorage<string>("race", {
        initialValue: "initial",
        storage,
      });

      return (
        <button data-testid="race" type="button" onClick={() => setValue("local")}>
          {value}:{String(hasHydrated)}
        </button>
      );
    }

    renderWithStorage(<Consumer />, storage);

    await waitFor(() => expect(screen.getByTestId("race")).toHaveTextContent("initial:false"));

    fireEvent.click(screen.getByTestId("race"));

    await act(async () => {
      deferred.resolveNextRead();
      await Promise.resolve();
    });

    await waitFor(() => expect(screen.getByTestId("race")).toHaveTextContent("local:true"));
  });

  it("migrates persisted values and stores the migrated payload", async () => {
    const storage = createStorage({ storage: createMemoryStorage() });
    storage.setItem("migration", {
      state: { value: { count: 1 }, version: 1 },
      version: 1,
    });

    function Consumer() {
      const [value] = useStorage<{ count: number }>("migration", {
        initialValue: { count: 0 },
        migrations: {
          1: (data: { count: number }) => ({ count: data.count + 1 }),
        },
        storage,
        version: 2,
      });

      return <div data-testid="migration">{value.count}</div>;
    }

    renderWithStorage(<Consumer />, storage);

    await waitFor(() => expect(screen.getByTestId("migration")).toHaveTextContent("2"));

    await expect(Promise.resolve(storage.getItem("migration"))).resolves.toEqual({
      state: { value: { count: 2 }, version: 2 },
      version: 2,
    });
  });

  it("applies value-carrying adapter subscriptions when sync is enabled", async () => {
    const storage = createStorage({ storage: createMemoryStorage() });

    function Consumer() {
      const [value] = useStorage<string>("synced", {
        initialValue: "initial",
        storage,
        sync: true,
      });

      return <div data-testid="synced">{value}</div>;
    }

    renderWithStorage(<Consumer />, storage);

    act(() => {
      storage.setItem("synced", {
        state: { value: "external", version: 1 },
        version: 1,
      });
    });

    await waitFor(() => expect(screen.getByTestId("synced")).toHaveTextContent("external"));
  });

  it("does not hydrate when disabled", async () => {
    const rawStorage: AbstractStorage = {
      getItem: vi.fn(() => Promise.resolve(null)),
      setItem: vi.fn(),
      removeItem: vi.fn(),
    };
    const storage = createAsyncStorage({ storage: rawStorage });

    function Consumer() {
      const [value, , hasHydrated] = useStorage<string>("disabled", {
        enabled: false,
        initialValue: "initial",
        storage,
      });

      return (
        <div data-testid="disabled">
          {value}:{String(hasHydrated)}
        </div>
      );
    }

    renderWithStorage(<Consumer />, storage);

    await Promise.resolve();

    expect(screen.getByTestId("disabled")).toHaveTextContent("initial:false");
    expect(rawStorage.getItem).not.toHaveBeenCalled();
  });
});
