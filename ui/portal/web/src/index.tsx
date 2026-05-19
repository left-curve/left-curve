import ReactDOM from "react-dom/client";
import { App } from "./app";
import { router } from "./app.router";
import { notifyUpdate } from "./app.updater";
import * as ReactScan from "react-scan";

import * as Sentry from "@sentry/react";

const SENTRY_DSN = import.meta.env.PUBLIC_SENTRY_DSN;
const SENTRY_ENV = import.meta.env.PUBLIC_SENTRY_ENVIRONMENT;

if (process.env.NODE_ENV === "development") ReactScan.start();

if (SENTRY_DSN && SENTRY_ENV) {
  Sentry.init({
    dsn: SENTRY_DSN,
    environment: SENTRY_ENV,
    integrations: (defaultIntegrations) =>
      defaultIntegrations
        .filter((integration) => integration.name !== "GlobalHandlers")
        .concat([
          Sentry.contextLinesIntegration(),
          Sentry.httpClientIntegration(),
          Sentry.tanstackRouterBrowserTracingIntegration(router),
        ]),
    tracesSampleRate: 0.5,
    tracePropagationTargets: [/^https:\/\/.+\.dango\.zone/],
    replaysOnErrorSampleRate: 0.5,
    maxValueLength: 5000,
  });
}

if (!window.location.origin.includes("localhost") && "serviceWorker" in navigator) {
  const initiallyControlled = !!navigator.serviceWorker.controller;
  let refreshing = false;
  navigator.serviceWorker.addEventListener("controllerchange", () => {
    if (!initiallyControlled) return;
    if (refreshing) return;
    refreshing = true;
    window.location.reload();
  });

  navigator.serviceWorker.register("/service-worker.js").then((registration) => {
    const handleInstalledWorker = (worker: ServiceWorker) => {
      void getWorkerCommit(worker).then((swCommit) => {
        if (swCommit && swCommit === import.meta.env.GIT_COMMIT) {
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
