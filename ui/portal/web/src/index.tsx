import ReactDOM from "react-dom/client";
import { App } from "./app";
import { router } from "./app.router";
import { notifyUpdate } from "./app.updater";
import * as ReactScan from "react-scan";

import * as Sentry from "@sentry/react";

const SENTRY_DSN = import.meta.env.PUBLIC_SENTRY_DSN;
const SENTRY_ENV = import.meta.env.PUBLIC_SENTRY_ENVIRONMENT;

if (process.env.NODE_ENV === "development") ReactScan.start();

const WALLET_ADDRESS_RE = /0x[a-fA-F0-9]{40}/g;

const scrubWallets = (value: unknown, seen: WeakSet<object> = new WeakSet()): unknown => {
  if (typeof value === "string") return value.replace(WALLET_ADDRESS_RE, "[wallet]");
  if (!value || typeof value !== "object") return value;
  if (seen.has(value)) return value;
  seen.add(value);
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) value[i] = scrubWallets(value[i], seen);
    return value;
  }
  for (const key of Object.keys(value)) {
    (value as Record<string, unknown>)[key] = scrubWallets(
      (value as Record<string, unknown>)[key],
      seen,
    );
  }
  return value;
};

const recentEvents = new Map<string, number>();
const DEDUP_WINDOW_MS = 30000;

if (SENTRY_DSN && SENTRY_ENV) {
  Sentry.init({
    dsn: SENTRY_DSN,
    environment: SENTRY_ENV,
    integrations: (defaultIntegrations) =>
      defaultIntegrations
        .filter((integration) => integration.name !== "GlobalHandlers")
        .concat([
          Sentry.contextLinesIntegration(),
          Sentry.tanstackRouterBrowserTracingIntegration(router),
        ]),
    tracesSampleRate: 0.5,
    tracePropagationTargets: [/^https:\/\/.+\.dango\.zone/],
    replaysOnErrorSampleRate: 0.5,
    maxValueLength: 5000,
    ignoreErrors: [/Error invoking postEvent: Java object is gone/i, /Telegram\.WebApp/i],
    beforeSend: (event) => {
      if (event.message) event.message = scrubWallets(event.message) as string;
      if (event.exception?.values) {
        for (const value of event.exception.values) {
          if (value.value) value.value = scrubWallets(value.value) as string;
        }
      }
      if (event.request?.url) event.request.url = scrubWallets(event.request.url) as string;
      if (event.extra) event.extra = scrubWallets(event.extra) as typeof event.extra;
      if (event.contexts) event.contexts = scrubWallets(event.contexts) as typeof event.contexts;

      const exc = event.exception?.values?.[0];
      const fingerprint = `${exc?.type ?? ""}:${exc?.value?.slice(0, 80) ?? ""}`;
      const now = Date.now();
      const lastSeen = recentEvents.get(fingerprint);
      if (lastSeen !== undefined && now - lastSeen < DEDUP_WINDOW_MS) return null;
      recentEvents.set(fingerprint, now);
      return event;
    },
  });
}

if (!window.location.origin.includes("localhost") && "serviceWorker" in navigator) {
  const initiallyControlled = !!navigator.serviceWorker.controller;
  let silentActivation = false;
  let refreshing = false;
  navigator.serviceWorker.addEventListener("controllerchange", () => {
    if (!initiallyControlled) return;
    if (silentActivation) {
      silentActivation = false;
      return;
    }
    if (refreshing) return;
    refreshing = true;
    window.location.reload();
  });

  navigator.serviceWorker.register("/service-worker.js").then((registration) => {
    const handleInstalledWorker = (worker: ServiceWorker) => {
      void getWorkerCommit(worker).then((swCommit) => {
        if (swCommit && swCommit === import.meta.env.GIT_COMMIT) {
          silentActivation = true;
          worker.postMessage({ type: "SKIP_WAITING" });
        } else {
          notifyUpdate(registration);
        }
      });
    };

    if (registration.waiting) handleInstalledWorker(registration.waiting);

    registration.addEventListener("updatefound", () => {
      const newWorker = registration.installing;
      if (!newWorker) return;
      newWorker.addEventListener("statechange", () => {
        if (newWorker.state === "installed" && navigator.serviceWorker.controller) {
          handleInstalledWorker(newWorker);
        }
      });
    });

    const intervalId = window.setInterval(() => registration.update(), 60 * 60 * 1000);
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") registration.update();
    };
    document.addEventListener("visibilitychange", onVisibilityChange);

    window.addEventListener("beforeunload", () => {
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    });
  });
}

function getWorkerCommit(worker: ServiceWorker): Promise<string | null> {
  return new Promise((resolve) => {
    const channel = new MessageChannel();
    const timer = window.setTimeout(() => resolve(null), 1000);
    channel.port1.onmessage = (event) => {
      window.clearTimeout(timer);
      resolve(typeof event.data?.commit === "string" ? event.data.commit : null);
    };
    worker.postMessage({ type: "GET_COMMIT" }, [channel.port2]);
  });
}

const container = document.getElementById("root");
if (!container) throw new Error("No root element found");

const root = ReactDOM.createRoot(container);
root.render(<App />);
