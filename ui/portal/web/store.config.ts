import {
  createAsyncStorage,
  createConfig,
  createTransport,
  passkey,
  session,
  privy,
} from "@left-curve/store";
import { captureException, withScope } from "@sentry/react";
import { createIndexedDBStorage } from "./storage.config";
import { coins } from "@left-curve/foundation/coins";

import type { Config } from "@left-curve/store/types";
import { serializeJson } from "@left-curve/encoding";

import { PRIVY_APP_ID, PRIVY_CLIENT_ID } from "~/constants";

const chain = window.dango.chain;

export const config: Config = createConfig({
  multiInjectedProviderDiscovery: true,
  chain,
  version: 2,
  transport: createTransport(`${chain.url}/graphql`, { batch: true, polling: false, lazy: false }),
  coins,
  connectors: [
    passkey(),
    session(),
    privy({
      appId: PRIVY_APP_ID as string,
      clientId: PRIVY_CLIENT_ID as string,
      poster: (url) => {
        const existIframe = document.getElementById("privy-iframe");
        if (existIframe) {
          const iframeWindow = (existIframe as HTMLIFrameElement).contentWindow!;
          return {
            reload: () => iframeWindow.location.reload(),
            postMessage: (message, targetOrigin, transfer) =>
              iframeWindow.postMessage(message, targetOrigin, transfer ? [transfer] : undefined),
          };
        }

        const iframe = window.document.createElement("iframe");
        iframe.style.display = "none";
        iframe.src = url;
        iframe.id = "privy-iframe";
        window.document.body.appendChild(iframe);
        const iframeWindow = (iframe as HTMLIFrameElement).contentWindow!;

        return {
          reload: () => iframeWindow.location.reload(),
          postMessage: (message, targetOrigin, transfer) =>
            iframeWindow.postMessage(message, targetOrigin, transfer ? [transfer] : undefined),
        };
      },
      listener: (onMessage) => {
        window.addEventListener("message", (event: MessageEvent) => {
          if (event.origin !== "https://auth.privy.io") return;
          try {
            onMessage(event.data);
          } catch (err) {
            console.error("Error handling iframe message:", err);
          }
        });
      },
    }),
  ],
  storage: createAsyncStorage({ storage: createIndexedDBStorage() }),
  onError: (e) => {
    if (
      Array.isArray(e) &&
      e.every((err: { message?: string }) => err.message?.includes("data not found"))
    )
      return;

    const errorName = e instanceof Error ? e.name : undefined;
    const cause = e instanceof Error ? (e.cause as { name?: string } | undefined) : undefined;

    if (errorName === "TimeoutError") return;
    if (cause?.name === "AbortError") return;
    if (
      cause?.name === "TypeError" &&
      typeof navigator !== "undefined" &&
      navigator.onLine === false
    ) {
      return;
    }
    if (
      errorName === "HttpRequestError" &&
      e instanceof Error &&
      /data not found/.test(String(e.message))
    ) {
      return;
    }

    const schemaDriftRe = /(Unknown argument|Cannot query field)/;
    const causeErrors = (e as { cause?: { errors?: { message?: string }[] } } | undefined)?.cause
      ?.errors;
    const matchesSchemaDrift =
      (e instanceof Error && schemaDriftRe.test(e.message)) ||
      (Array.isArray(causeErrors) &&
        causeErrors.some((err) => err?.message && schemaDriftRe.test(err.message)));

    if (matchesSchemaDrift) {
      // TODO(PORTAL-WEBSITE-6VB): wire to "new version available" updater once available.
      console.warn("Schema drift detected — client likely needs to update.", e);
      return;
    }

    let finalError: Error;
    const m = serializeJson(e);

    if (Array.isArray(e) && e[0]?.message) {
      finalError = new Error(`GraphQLWS Error: ${e[0].message} (${m})`);
      captureException(finalError);
    } else if (e instanceof Event) {
      const code = (e as CloseEvent).code ?? null;
      if ("code" in e) {
        finalError = new Error(`WebSocket closed: (${m})`);
      } else {
        finalError = new Error(`WebSocket connection failed (${m})`);
      }
      withScope((scope) => {
        scope.setLevel("warning");
        scope.setContext("websocket", { code });
        captureException(finalError);
      });
    } else if (e instanceof Error) {
      finalError = e;
      captureException(finalError);
    } else {
      finalError = new Error(`Unknown Error (${m})`);
      captureException(finalError);
    }
  },
});
