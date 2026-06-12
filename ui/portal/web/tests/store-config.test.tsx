import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const storeConfigMocks = vi.hoisted(() => {
  const asyncStorage = { kind: "async-storage" };
  const config = { kind: "config" };
  const indexedDBStorage = { kind: "indexed-db-storage" };

  return {
    asyncStorage,
    coins: { kind: "coins" },
    config,
    createAsyncStorage: vi.fn(() => asyncStorage),
    createConfig: vi.fn(() => config),
    createIndexedDBStorage: vi.fn(() => indexedDBStorage),
    createTransport: vi.fn((url: string, options: unknown) => ({
      options,
      url,
    })),
    debug: vi.fn(() => ({ id: "debug", type: "debug" })),
    passkey: vi.fn(() => ({ id: "passkey", type: "passkey" })),
    privy: vi.fn((options: unknown) => ({ id: "privy", options, type: "privy" })),
    reportStoreError: vi.fn(),
    session: vi.fn(() => ({ id: "session", type: "session" })),
  };
});

vi.mock("@left-curve/store", () => ({
  createAsyncStorage: storeConfigMocks.createAsyncStorage,
  createConfig: storeConfigMocks.createConfig,
  createTransport: storeConfigMocks.createTransport,
  debug: storeConfigMocks.debug,
  passkey: storeConfigMocks.passkey,
  privy: storeConfigMocks.privy,
  session: storeConfigMocks.session,
}));

vi.mock("@left-curve/foundation/coins", () => ({
  coins: storeConfigMocks.coins,
}));

vi.mock("../storage.config", () => ({
  createIndexedDBStorage: storeConfigMocks.createIndexedDBStorage,
}));

vi.mock("~/app.sentry", () => ({
  reportStoreError: storeConfigMocks.reportStoreError,
}));

vi.mock("~/constants", () => ({
  PRIVY_APP_ID: "privy-app-id",
  PRIVY_CLIENT_ID: "privy-client-id",
}));

type PrivyOptions = {
  appId: string;
  clientId: string;
  listener: (onMessage: (message: unknown) => void) => void;
  poster: (url: string) => {
    postMessage: (message: unknown, targetOrigin: string, transfer?: Transferable) => void;
    reload: () => void;
  };
};

function installDangoRuntime() {
  Object.defineProperty(window, "dango", {
    configurable: true,
    value: {
      chain: {
        id: "dango-dev-1",
        name: "Devnet",
        url: "https://rpc.dango.test",
      },
    },
  });
}

async function importStoreConfig() {
  vi.resetModules();
  return import("../store.config");
}

function getPrivyOptions(): PrivyOptions {
  const options = storeConfigMocks.privy.mock.calls[0]?.[0];
  if (!options) throw new Error("Expected store config to create a Privy connector");
  return options as PrivyOptions;
}

describe("portal store config", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
    installDangoRuntime();
  });

  afterEach(() => {
    document.body.innerHTML = "";
    vi.clearAllMocks();
  });

  it("builds the store config from the runtime Dango chain and shared environment boundaries", async () => {
    const { config } = await importStoreConfig();
    const transport = storeConfigMocks.createTransport.mock.results[0]?.value;
    const indexedStorage = storeConfigMocks.createIndexedDBStorage.mock.results[0]?.value;
    const asyncStorage = storeConfigMocks.createAsyncStorage.mock.results[0]?.value;

    expect(config).toBe(storeConfigMocks.config);
    expect(storeConfigMocks.createTransport).toHaveBeenCalledWith(
      "https://rpc.dango.test/graphql",
      {
        batch: true,
        lazy: false,
        polling: false,
      },
    );
    expect(storeConfigMocks.createAsyncStorage).toHaveBeenCalledWith({
      storage: indexedStorage,
    });
    expect(storeConfigMocks.passkey).toHaveBeenCalledOnce();
    expect(storeConfigMocks.session).toHaveBeenCalledOnce();
    expect(storeConfigMocks.debug).toHaveBeenCalledOnce();
    expect(storeConfigMocks.privy).toHaveBeenCalledWith(
      expect.objectContaining({
        appId: "privy-app-id",
        clientId: "privy-client-id",
        listener: expect.any(Function),
        poster: expect.any(Function),
      }),
    );
    expect(storeConfigMocks.createConfig).toHaveBeenCalledWith({
      chain: window.dango.chain,
      coins: storeConfigMocks.coins,
      connectors: [
        { id: "passkey", type: "passkey" },
        { id: "session", type: "session" },
        { id: "debug", type: "debug" },
        {
          id: "privy",
          options: expect.objectContaining({
            appId: "privy-app-id",
            clientId: "privy-client-id",
          }),
          type: "privy",
        },
      ],
      multiInjectedProviderDiscovery: true,
      onError: storeConfigMocks.reportStoreError,
      storage: asyncStorage,
      transport,
      version: 2,
    });
  });

  it("creates and reuses the hidden Privy iframe poster with transferable messages", async () => {
    await importStoreConfig();
    const { poster } = getPrivyOptions();
    const firstPoster = poster("https://auth.privy.io/iframe");
    const iframe = document.getElementById("privy-iframe") as HTMLIFrameElement | null;

    expect(iframe).toBeInstanceOf(HTMLIFrameElement);
    expect(iframe).toHaveAttribute("src", "https://auth.privy.io/iframe");
    expect(iframe?.style.display).toBe("none");

    const postMessage = vi
      .spyOn(iframe!.contentWindow!, "postMessage")
      .mockImplementation(() => undefined);
    const transfer = new MessageChannel().port1;

    firstPoster.postMessage({ type: "hello" }, "https://auth.privy.io", transfer);

    expect(postMessage).toHaveBeenCalledWith({ type: "hello" }, "https://auth.privy.io", [
      transfer,
    ]);

    const secondPoster = poster("https://auth.privy.io/new-url");
    secondPoster.postMessage({ type: "again" }, "https://auth.privy.io");

    expect(document.querySelectorAll("#privy-iframe")).toHaveLength(1);
    expect(postMessage).toHaveBeenLastCalledWith(
      { type: "again" },
      "https://auth.privy.io",
      undefined,
    );
  });

  it("accepts only Privy iframe messages and contains connector handler failures", async () => {
    await importStoreConfig();
    const { listener } = getPrivyOptions();
    const onMessage = vi.fn();
    const error = new Error("handler failed");
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined);

    listener(onMessage);
    window.dispatchEvent(
      new MessageEvent("message", {
        data: { type: "ignored" },
        origin: "https://example.invalid",
      }),
    );
    window.dispatchEvent(
      new MessageEvent("message", {
        data: { type: "privy-ready" },
        origin: "https://auth.privy.io",
      }),
    );
    onMessage.mockImplementationOnce(() => {
      throw error;
    });
    window.dispatchEvent(
      new MessageEvent("message", {
        data: { type: "privy-error-path" },
        origin: "https://auth.privy.io",
      }),
    );

    expect(onMessage).toHaveBeenCalledTimes(2);
    expect(onMessage).toHaveBeenNthCalledWith(1, { type: "privy-ready" });
    expect(onMessage).toHaveBeenNthCalledWith(2, { type: "privy-error-path" });
    expect(consoleError).toHaveBeenCalledWith("Error handling iframe message:", error);

    consoleError.mockRestore();
  });
});
