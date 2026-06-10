import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type QueryKey = readonly unknown[];
type InvalidateMessage = {
  type: "invalidate";
  keys: QueryKey[];
};

class MockBroadcastChannel {
  static instances: MockBroadcastChannel[] = [];

  messages: unknown[] = [];
  name: string;
  onmessage: ((event: { data: unknown }) => void) | null = null;

  constructor(name: string) {
    this.name = name;
    MockBroadcastChannel.instances.push(this);
  }

  postMessage(message: unknown) {
    this.messages.push(message);
  }

  emit(message: InvalidateMessage | { type: string }) {
    this.onmessage?.({ data: message });
  }

  close() {}
}

async function importFreshQueryClient() {
  vi.resetModules();
  vi.stubGlobal("BroadcastChannel", MockBroadcastChannel);
  const module = await import("../src/queryClient");
  const channel = MockBroadcastChannel.instances.at(-1);
  if (!channel) throw new Error("Expected queryClient to create a BroadcastChannel");
  return {
    channel,
    queryClient: module.queryClient,
  };
}

describe("portal queryClient", () => {
  beforeEach(() => {
    MockBroadcastChannel.instances = [];
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("uses the shared query invalidation channel and frontend query defaults", async () => {
    const { channel, queryClient } = await importFreshQueryClient();

    expect(channel.name).toBe("dango.queries");
    expect(queryClient.getDefaultOptions().queries).toMatchObject({
      refetchOnWindowFocus: false,
      retry: 0,
    });
  });

  it("broadcasts mutation invalidate keys after a mutation settles", async () => {
    const { channel, queryClient } = await importFreshQueryClient();

    const mutation = queryClient.getMutationCache().build(queryClient, {
      meta: {
        invalidateKeys: [
          ["balances", "0x616c696365000000000000000000000000000000"],
          ["quests", "alice"],
        ],
      },
      mutationFn: async () => ({ ok: true }),
    });

    await mutation.execute();

    expect(channel.messages).toEqual([
      {
        keys: [
          ["balances", "0x616c696365000000000000000000000000000000"],
          ["quests", "alice"],
        ],
        type: "invalidate",
      },
    ]);
  });

  it("broadcasts mutation invalidate keys when a mutation rejects", async () => {
    const { channel, queryClient } = await importFreshQueryClient();
    const mutationError = new Error("mutation failed");

    const mutation = queryClient.getMutationCache().build(queryClient, {
      meta: {
        invalidateKeys: [["balances", "0x626f620000000000000000000000000000000000"]],
      },
      mutationFn: async () => {
        throw mutationError;
      },
    });

    await expect(mutation.execute()).rejects.toThrow(mutationError);

    expect(channel.messages).toEqual([
      {
        keys: [["balances", "0x626f620000000000000000000000000000000000"]],
        type: "invalidate",
      },
    ]);
  });

  it("does not broadcast mutation settlements without invalidate keys", async () => {
    const { channel, queryClient } = await importFreshQueryClient();

    const mutation = queryClient.getMutationCache().build(queryClient, {
      mutationFn: async () => ({ ok: true }),
    });

    await mutation.execute();

    expect(channel.messages).toEqual([]);
  });

  it("preserves backend user index zero in cross-tab invalidation keys", async () => {
    const { channel, queryClient } = await importFreshQueryClient();
    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

    const mutation = queryClient.getMutationCache().build(queryClient, {
      meta: {
        invalidateKeys: [
          ["quests", 0],
          ["boxes", 0],
        ],
      },
      mutationFn: async () => ({ ok: true }),
    });

    await mutation.execute();

    expect(channel.messages).toEqual([
      {
        keys: [
          ["quests", 0],
          ["boxes", 0],
        ],
        type: "invalidate",
      },
    ]);

    channel.emit({
      keys: [
        ["boosters", 0],
        ["user", 0],
      ],
      type: "invalidate",
    });

    expect(invalidateQueries).toHaveBeenNthCalledWith(1, { queryKey: ["boosters", 0] });
    expect(invalidateQueries).toHaveBeenNthCalledWith(2, { queryKey: ["user", 0] });
  });

  it("invalidates every query key from incoming invalidation messages", async () => {
    const { channel, queryClient } = await importFreshQueryClient();
    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

    channel.emit({
      keys: [
        ["balances", "0x616c696365000000000000000000000000000000"],
        ["quests", "alice"],
      ],
      type: "invalidate",
    });

    expect(invalidateQueries).toHaveBeenNthCalledWith(1, {
      queryKey: ["balances", "0x616c696365000000000000000000000000000000"],
    });
    expect(invalidateQueries).toHaveBeenNthCalledWith(2, {
      queryKey: ["quests", "alice"],
    });

    invalidateQueries.mockClear();
    channel.emit({ type: "noop" });

    expect(invalidateQueries).not.toHaveBeenCalled();
  });
});
