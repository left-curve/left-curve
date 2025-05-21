import ReactDOM from "react-dom/client";
import { App } from "./app";
import { router } from "./app.router";

import * as Sentry from "@sentry/react";

const SENTRY_DSN = import.meta.env.PUBLIC_SENTRY_DSN;

if (SENTRY_DSN) {
  Sentry.init({
    dsn: SENTRY_DSN,
    integrations: (defaultIntegrations) =>
      defaultIntegrations
        .filter((integration) => integration.name !== "GlobalHandlers")
        .concat([
          Sentry.contextLinesIntegration(),
          Sentry.httpClientIntegration(),
          Sentry.tanstackRouterBrowserTracingIntegration(router),
        ]),
    tracesSampleRate: 0.5,
    tracePropagationTargets: [/^https:\/\/testnet\.dango\.exchange\//],
    replaysOnErrorSampleRate: 0.5,
    maxValueLength: 5000,
  });
}

const container = document.getElementById("root");
if (!container) throw new Error("No root element found");

const root = ReactDOM.createRoot(container);
root.render(<App />);
