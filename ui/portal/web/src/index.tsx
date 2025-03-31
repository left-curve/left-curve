import ReactDOM from "react-dom/client";
import { App } from "./app";
import { router } from "./app.router";

import * as Sentry from "@sentry/react";

Sentry.init({
  dsn: import.meta.env.PUBLIC_SENTRY_DSN,
  integrations: [
    Sentry.browserTracingIntegration(),
    Sentry.replayIntegration(),
    Sentry.tanstackRouterBrowserTracingIntegration(router),
  ],
  tracesSampleRate: 1.0,
  tracePropagationTargets: ["devnet.dango.exchange"],
  replaysOnErrorSampleRate: 1.0,
});

const container = document.getElementById("root");
if (!container) throw new Error("No root element found");

const root = ReactDOM.createRoot(container);
root.render(<App />);
