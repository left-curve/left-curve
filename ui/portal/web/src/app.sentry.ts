import * as Sentry from "@sentry/react";

import { serializeJson } from "@left-curve/encoding";

import { router } from "./app.router";

const WALLET_ADDRESS_RE = /0x[a-fA-F0-9]{40}/g;
const scrubWallets = (value: string): string => value.replace(WALLET_ADDRESS_RE, "[wallet]");

const SCHEMA_DRIFT_RE = /(Unknown argument|Cannot query field)/;

const isIgnorable = (e: unknown): boolean => {
  if (
    Array.isArray(e) &&
    e.every((err: { message?: string }) => err.message?.includes("data not found"))
  ) {
    return true;
  }
  if (!(e instanceof Error)) return false;

  const cause = e.cause as { name?: string } | undefined;

  if (e.name === "TimeoutError") return true;
  if (cause?.name === "AbortError") return true;
  if (cause?.name === "TypeError" && typeof navigator !== "undefined" && !navigator.onLine) {
    return true;
  }
  if (e.name === "HttpRequestError" && /data not found/.test(e.message)) return true;
  return false;
};

const isSchemaDrift = (e: unknown): boolean => {
  if (e instanceof Error && SCHEMA_DRIFT_RE.test(e.message)) return true;
  const causeErrors = (e as { cause?: { errors?: { message?: string }[] } } | undefined)?.cause
    ?.errors;
  return (
    Array.isArray(causeErrors) &&
    causeErrors.some((err) => typeof err?.message === "string" && SCHEMA_DRIFT_RE.test(err.message))
  );
};

export const initSentry = (dsn: string, environment: string): void => {
  Sentry.init({
    dsn,
    environment,
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
      if (event.message) event.message = scrubWallets(event.message);
      if (event.exception?.values) {
        for (const value of event.exception.values) {
          if (value.value) value.value = scrubWallets(value.value);
        }
      }
      if (event.request?.url) event.request.url = scrubWallets(event.request.url);
      return event;
    },
  });
};

export const reportStoreError = (e: unknown): void => {
  if (isIgnorable(e)) return;

  if (isSchemaDrift(e)) {
    Sentry.withScope((scope) => {
      scope.setLevel("warning");
      scope.setFingerprint(["schema-drift"]);
      scope.setContext("schema_drift", {
        message: e instanceof Error ? e.message : String(e),
      });
      Sentry.captureMessage("Schema drift detected — client likely needs to update.");
    });
    return;
  }

  const m = serializeJson(e);

  if (Array.isArray(e) && e[0]?.message) {
    Sentry.captureException(new Error(`GraphQLWS Error: ${e[0].message} (${m})`));
    return;
  }

  if (e instanceof Event) {
    const code = (e as CloseEvent).code ?? null;
    const message = "code" in e ? `WebSocket closed: (${m})` : `WebSocket connection failed (${m})`;
    Sentry.withScope((scope) => {
      scope.setLevel("warning");
      scope.setContext("websocket", { code });
      Sentry.captureException(new Error(message));
    });
    return;
  }

  Sentry.captureException(e instanceof Error ? e : new Error(`Unknown Error (${m})`));
};
