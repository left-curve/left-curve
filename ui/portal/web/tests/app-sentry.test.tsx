import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const sentryMocks = vi.hoisted(() => ({
  captureException: vi.fn(),
  captureMessage: vi.fn(),
  contextLinesIntegration: vi.fn(() => ({ name: "ContextLines" })),
  init: vi.fn(),
  router: { id: "router" },
  scope: {
    setContext: vi.fn(),
    setFingerprint: vi.fn(),
    setLevel: vi.fn(),
  },
  tanstackRouterBrowserTracingIntegration: vi.fn(() => ({ name: "TanstackRouterTracing" })),
  withScope: vi.fn((callback: (scope: unknown) => void) => callback(sentryMocks.scope)),
}));

vi.mock("@sentry/react", () => ({
  captureException: sentryMocks.captureException,
  captureMessage: sentryMocks.captureMessage,
  contextLinesIntegration: sentryMocks.contextLinesIntegration,
  init: sentryMocks.init,
  tanstackRouterBrowserTracingIntegration: sentryMocks.tanstackRouterBrowserTracingIntegration,
  withScope: sentryMocks.withScope,
}));

vi.mock("../src/app.router", () => ({
  router: sentryMocks.router,
}));

async function loadSentryModule() {
  return import("../src/app.sentry");
}

describe("app Sentry integration", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("initializes Sentry with router tracing and scrubs wallet addresses from events", async () => {
    const { initSentry } = await loadSentryModule();
    const wallet = "0x1111111111111111111111111111111111111111";

    initSentry("https://sentry.example/dsn", "production");

    expect(sentryMocks.init).toHaveBeenCalledWith(
      expect.objectContaining({
        dsn: "https://sentry.example/dsn",
        environment: "production",
        maxValueLength: 5000,
        replaysOnErrorSampleRate: 0.5,
        tracesSampleRate: 0.5,
      }),
    );

    const options = sentryMocks.init.mock.calls[0][0];
    const integrations = options.integrations([
      { name: "GlobalHandlers" },
      { name: "DefaultIntegration" },
    ]);

    expect(integrations).toEqual([
      { name: "DefaultIntegration" },
      { name: "ContextLines" },
      { name: "TanstackRouterTracing" },
    ]);
    expect(sentryMocks.tanstackRouterBrowserTracingIntegration).toHaveBeenCalledWith(
      sentryMocks.router,
    );

    const event = options.beforeSend({
      exception: {
        values: [{ value: `transfer failed for ${wallet}` }],
      },
      message: `wallet ${wallet} failed`,
      request: {
        url: `https://portal.example/account/${wallet}`,
      },
    });

    expect(event.message).toBe("wallet [wallet] failed");
    expect(event.exception.values[0].value).toBe("transfer failed for [wallet]");
    expect(event.request.url).toBe("https://portal.example/account/[wallet]");
  });

  it("scrubs every wallet address occurrence before sending Sentry events", async () => {
    const { initSentry } = await loadSentryModule();
    const sender = "0x1111111111111111111111111111111111111111";
    const recipient = "0x2222222222222222222222222222222222222222";

    initSentry("https://sentry.example/dsn", "production");

    const options = sentryMocks.init.mock.calls[0][0];
    const event = options.beforeSend({
      exception: {
        values: [{ value: `transfer from ${sender} to ${recipient} failed` }],
      },
      message: `wallets ${sender} and ${recipient}`,
      request: {
        url: `https://portal.example/transfer?from=${sender}&to=${recipient}`,
      },
    });

    expect(event.message).toBe("wallets [wallet] and [wallet]");
    expect(event.exception.values[0].value).toBe("transfer from [wallet] to [wallet] failed");
    expect(event.request.url).toBe("https://portal.example/transfer?from=[wallet]&to=[wallet]");
  });

  it("reports backend schema drift as a warning with a stable fingerprint", async () => {
    const { reportStoreError } = await loadSentryModule();
    const error = new Error("Cannot query field missing_field on type Query");

    reportStoreError(error);

    expect(sentryMocks.withScope).toHaveBeenCalledOnce();
    expect(sentryMocks.scope.setLevel).toHaveBeenCalledWith("warning");
    expect(sentryMocks.scope.setFingerprint).toHaveBeenCalledWith(["schema-drift"]);
    expect(sentryMocks.scope.setContext).toHaveBeenCalledWith("schema_drift", {
      message: error.message,
    });
    expect(sentryMocks.captureMessage).toHaveBeenCalledWith(
      "Schema drift detected — client likely needs to update.",
    );
    expect(sentryMocks.captureException).not.toHaveBeenCalled();
  });

  it("reports nested GraphQL schema drift responses as warnings", async () => {
    const { reportStoreError } = await loadSentryModule();
    const error = {
      cause: {
        errors: [{ message: "Unknown argument cursor on field Query.transactions" }],
      },
    };

    reportStoreError(error);

    expect(sentryMocks.withScope).toHaveBeenCalledOnce();
    expect(sentryMocks.scope.setLevel).toHaveBeenCalledWith("warning");
    expect(sentryMocks.scope.setFingerprint).toHaveBeenCalledWith(["schema-drift"]);
    expect(sentryMocks.scope.setContext).toHaveBeenCalledWith("schema_drift", {
      message: String(error),
    });
    expect(sentryMocks.captureMessage).toHaveBeenCalledWith(
      "Schema drift detected — client likely needs to update.",
    );
    expect(sentryMocks.captureException).not.toHaveBeenCalled();
  });

  it("drops expected backend not-found and timeout noise", async () => {
    const { reportStoreError } = await loadSentryModule();

    reportStoreError([{ message: "data not found" }, { message: "data not found" }]);
    reportStoreError(Object.assign(new Error("request timed out"), { name: "TimeoutError" }));
    reportStoreError(
      Object.assign(new Error("data not found at requested path"), { name: "HttpRequestError" }),
    );

    expect(sentryMocks.captureException).not.toHaveBeenCalled();
    expect(sentryMocks.captureMessage).not.toHaveBeenCalled();
    expect(sentryMocks.withScope).not.toHaveBeenCalled();
  });

  it("drops expected abort and offline request errors", async () => {
    const { reportStoreError } = await loadSentryModule();
    const abortError = new Error("request aborted", { cause: { name: "AbortError" } });
    const offlineError = new Error("failed to fetch", { cause: { name: "TypeError" } });

    Object.defineProperty(navigator, "onLine", {
      configurable: true,
      value: false,
    });

    reportStoreError(abortError);
    reportStoreError(offlineError);

    expect(sentryMocks.captureException).not.toHaveBeenCalled();
    expect(sentryMocks.captureMessage).not.toHaveBeenCalled();
    expect(sentryMocks.withScope).not.toHaveBeenCalled();
  });

  it("reports GraphQL websocket errors with the backend message and payload", async () => {
    const { reportStoreError } = await loadSentryModule();

    reportStoreError([{ message: "subscription rejected", code: "FORBIDDEN" }]);

    expect(sentryMocks.captureException).toHaveBeenCalledOnce();
    const error = sentryMocks.captureException.mock.calls[0][0];
    expect(error).toBeInstanceOf(Error);
    expect(error.message).toContain("GraphQLWS Error: subscription rejected");
    expect(error.message).toContain("FORBIDDEN");
    expect(sentryMocks.withScope).not.toHaveBeenCalled();
    expect(sentryMocks.captureMessage).not.toHaveBeenCalled();
  });

  it("reports websocket close and browser error events as warnings", async () => {
    const { reportStoreError } = await loadSentryModule();
    const closeEvent = Object.assign(new Event("close"), { code: 1006 });

    reportStoreError(closeEvent);

    expect(sentryMocks.withScope).toHaveBeenCalledOnce();
    expect(sentryMocks.scope.setLevel).toHaveBeenCalledWith("warning");
    expect(sentryMocks.scope.setContext).toHaveBeenCalledWith("websocket", { code: 1006 });
    expect(sentryMocks.captureException).toHaveBeenCalledOnce();
    expect(sentryMocks.captureException.mock.calls[0][0].message).toContain("WebSocket closed");

    vi.clearAllMocks();

    reportStoreError(new Event("error"));

    expect(sentryMocks.withScope).toHaveBeenCalledOnce();
    expect(sentryMocks.scope.setLevel).toHaveBeenCalledWith("warning");
    expect(sentryMocks.scope.setContext).toHaveBeenCalledWith("websocket", { code: null });
    expect(sentryMocks.captureException).toHaveBeenCalledOnce();
    expect(sentryMocks.captureException.mock.calls[0][0].message).toContain(
      "WebSocket connection failed",
    );
  });
});
