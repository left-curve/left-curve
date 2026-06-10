import { cleanup, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type Listener = () => void;

const entryMocks = vi.hoisted(() => ({
  createRoot: vi.fn(),
  initSentry: vi.fn(),
  notifyUpdate: vi.fn(),
  reactScanStart: vi.fn(),
  render: vi.fn(),
}));

vi.mock("react-dom/client", () => ({
  default: {
    createRoot: entryMocks.createRoot,
  },
}));

vi.mock("react-scan", () => ({
  start: entryMocks.reactScanStart,
}));

vi.mock("../src/app", async () => {
  const React = await import("react");

  return {
    App: () => React.createElement("div", { "data-testid": "portal-app" }),
  };
});

vi.mock("../src/app.sentry", () => ({
  initSentry: entryMocks.initSentry,
}));

vi.mock("../src/app.updater", () => ({
  notifyUpdate: entryMocks.notifyUpdate,
}));

function stubLocation(origin: string) {
  const reload = vi.fn();
  vi.stubGlobal("location", {
    href: `${origin}/`,
    origin,
    reload,
  });
  return reload;
}

function createWorker(commit: string | null) {
  const listeners: Record<string, Listener[]> = {};
  const worker = {
    addEventListener: vi.fn((type: string, listener: Listener) => {
      listeners[type] = [...(listeners[type] ?? []), listener];
    }),
    postMessage: vi.fn((message: { type?: string }, ports?: MessagePort[]) => {
      if (message.type !== "GET_COMMIT") return;
      queueMicrotask(() => {
        ports?.[0]?.postMessage({ commit });
      });
    }),
    state: "installed",
  };

  return {
    emit: (type: string) => listeners[type]?.forEach((listener) => listener()),
    worker: worker as unknown as ServiceWorker,
    workerMock: worker,
  };
}

function createRegistration(waiting?: ServiceWorker | null) {
  const listeners: Record<string, Listener[]> = {};
  const registration = {
    addEventListener: vi.fn((type: string, listener: Listener) => {
      listeners[type] = [...(listeners[type] ?? []), listener];
    }),
    installing: null as ServiceWorker | null,
    update: vi.fn(),
    waiting: waiting ?? null,
  };

  return {
    emit: (type: string) => listeners[type]?.forEach((listener) => listener()),
    registration: registration as unknown as ServiceWorkerRegistration,
    registrationMock: registration,
  };
}

function installServiceWorker(
  registration: ServiceWorkerRegistration,
  { controlled = true }: { controlled?: boolean } = {},
) {
  const listeners: Record<string, Listener[]> = {};
  const serviceWorker = {
    addEventListener: vi.fn((type: string, listener: Listener) => {
      listeners[type] = [...(listeners[type] ?? []), listener];
    }),
    controller: controlled ? ({} as ServiceWorker) : null,
    register: vi.fn().mockResolvedValue(registration),
  };

  Object.defineProperty(navigator, "serviceWorker", {
    configurable: true,
    value: serviceWorker,
  });

  return {
    emit: (type: string) => listeners[type]?.forEach((listener) => listener()),
    serviceWorker,
  };
}

async function loadEntrypoint() {
  await import("../src/index");
}

describe("app entrypoint", () => {
  beforeEach(() => {
    vi.resetModules();
    document.body.innerHTML = '<div id="root"></div>';
    entryMocks.createRoot.mockReturnValue({
      render: entryMocks.render,
    });
  });

  afterEach(() => {
    window.dispatchEvent(new Event("beforeunload"));
    cleanup();
    document.body.innerHTML = "";
    Reflect.deleteProperty(navigator, "serviceWorker");
    vi.clearAllMocks();
    vi.unstubAllEnvs();
    vi.unstubAllGlobals();
  });

  it("renders the app without registering a service worker on localhost", async () => {
    stubLocation("http://localhost:5173");
    const registration = createRegistration();
    const serviceWorker = installServiceWorker(registration.registration);

    await loadEntrypoint();

    expect(entryMocks.createRoot).toHaveBeenCalledWith(document.getElementById("root"));
    expect(entryMocks.render).toHaveBeenCalledOnce();
    expect(serviceWorker.serviceWorker.register).not.toHaveBeenCalled();
    expect(entryMocks.notifyUpdate).not.toHaveBeenCalled();
  });

  it("notifies controlled clients when a waiting worker has a different commit", async () => {
    const reload = stubLocation("https://portal.example");
    const waiting = createWorker("older-commit");
    const registration = createRegistration(waiting.worker);
    const serviceWorker = installServiceWorker(registration.registration);

    await loadEntrypoint();

    await waitFor(() => {
      expect(serviceWorker.serviceWorker.register).toHaveBeenCalledWith("/service-worker.js");
    });
    await waitFor(() => {
      expect(entryMocks.notifyUpdate).toHaveBeenCalledWith(registration.registration);
    });

    expect(waiting.workerMock.postMessage).toHaveBeenCalledWith(
      { type: "GET_COMMIT" },
      expect.any(Array),
    );

    serviceWorker.emit("controllerchange");

    expect(reload).toHaveBeenCalledOnce();

    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
    document.dispatchEvent(new Event("visibilitychange"));

    expect(registration.registrationMock.update).toHaveBeenCalledOnce();

    window.dispatchEvent(new Event("beforeunload"));
    document.dispatchEvent(new Event("visibilitychange"));

    expect(registration.registrationMock.update).toHaveBeenCalledOnce();
  });

  it("notifies controlled clients when a newly installed worker has a different commit", async () => {
    stubLocation("https://portal.example");
    const nextWorker = createWorker("newer-commit");
    const registration = createRegistration();
    installServiceWorker(registration.registration);

    await loadEntrypoint();

    await waitFor(() => {
      expect(registration.registrationMock.addEventListener).toHaveBeenCalledWith(
        "updatefound",
        expect.any(Function),
      );
    });
    expect(entryMocks.notifyUpdate).not.toHaveBeenCalled();

    registration.registrationMock.installing = nextWorker.worker;
    registration.emit("updatefound");

    expect(nextWorker.workerMock.addEventListener).toHaveBeenCalledWith(
      "statechange",
      expect.any(Function),
    );
    expect(entryMocks.notifyUpdate).not.toHaveBeenCalled();

    nextWorker.emit("statechange");

    await waitFor(() => {
      expect(entryMocks.notifyUpdate).toHaveBeenCalledWith(registration.registration);
    });
    expect(nextWorker.workerMock.postMessage).toHaveBeenCalledWith(
      { type: "GET_COMMIT" },
      expect.any(Array),
    );
  });

  it("does not prompt or reload when the page receives its first service worker controller", async () => {
    const reload = stubLocation("https://portal.example");
    const nextWorker = createWorker("first-install-commit");
    const registration = createRegistration();
    const serviceWorker = installServiceWorker(registration.registration, { controlled: false });

    await loadEntrypoint();

    registration.registrationMock.installing = nextWorker.worker;
    registration.emit("updatefound");
    nextWorker.emit("statechange");
    serviceWorker.emit("controllerchange");

    expect(nextWorker.workerMock.addEventListener).toHaveBeenCalledWith(
      "statechange",
      expect.any(Function),
    );
    expect(nextWorker.workerMock.postMessage).not.toHaveBeenCalled();
    expect(entryMocks.notifyUpdate).not.toHaveBeenCalled();
    expect(reload).not.toHaveBeenCalled();
  });

  it("silently activates same-commit waiting workers without prompting or reloading", async () => {
    vi.stubEnv("GIT_COMMIT", "same-commit");
    const reload = stubLocation("https://portal.example");
    const waiting = createWorker("same-commit");
    const registration = createRegistration(waiting.worker);
    const serviceWorker = installServiceWorker(registration.registration);

    await loadEntrypoint();

    await waitFor(() => {
      expect(waiting.workerMock.postMessage).toHaveBeenCalledWith({ type: "SKIP_WAITING" });
    });

    serviceWorker.emit("controllerchange");

    expect(entryMocks.notifyUpdate).not.toHaveBeenCalled();
    expect(reload).not.toHaveBeenCalled();
  });
});
