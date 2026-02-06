// Trigger issue

import ReactDOM from "react-dom/client";
import { App } from "./app";
import { router } from "./app.router";
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

let refreshing = false;
navigator?.serviceWorker?.addEventListener("controllerchange", () => {
  if (!refreshing) {
    refreshing = true;
    window.location.reload();
  }
});

const container = document.getElementById("root");
if (!container) throw new Error("No root element found");

const root = ReactDOM.createRoot(container);
root.render(<App />);
