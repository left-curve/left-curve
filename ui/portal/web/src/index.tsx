import ReactDOM from "react-dom/client";
import { App } from "./app";
import { router } from "./app.router";

import * as Sentry from "@sentry/react";

const SENTRY_DNS = import.meta.env.PUBLIC_SENTRY_DSN;

if (SENTRY_DNS) {
  Sentry.init({
    dsn: SENTRY_DNS,
    integrations: [
      Sentry.httpClientIntegration(),
      Sentry.replayIntegration(),
      Sentry.tanstackRouterBrowserTracingIntegration(router),
    ],
    tracesSampleRate: 0.5,
    tracePropagationTargets: [/^https:\/\/devnet\.dango\.exchange\//],
    replaysOnErrorSampleRate: 0.5,
    maxValueLength: 5000,
  });
}

const container = document.getElementById("root");
if (!container) throw new Error("No root element found");

const root = ReactDOM.createRoot(container);
root.render(<App />);
