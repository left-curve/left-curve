import {
  createAsyncStorage,
  createConfig,
  graphql,
  passkey,
  session,
  privy,
} from "@left-curve/store";
import { captureException } from "@sentry/react";
import { createIndexedDBStorage } from "./storage.config";
import { coins } from "@left-curve/foundation/coins";

import type { Config } from "@left-curve/store/types";
import { serializeJson } from "@left-curve/dango/encoding";

import { PRIVY_APP_ID, PRIVY_CLIENT_ID } from "~/constants";

const chain = window.dango.chain;

export const config: Config = createConfig({
  multiInjectedProviderDiscovery: true,
  chain,
  version: 1,
  transport: graphql(`${chain.urls.indexer}/graphql`, { batch: true, lazy: false }),
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
    let finalError: Error;
    const m = serializeJson(e);

    if (Array.isArray(e) && e[0]?.message) {
      finalError = new Error(`GraphQLWS Error: ${e[0].message} (${m})`);
    } else if (e instanceof Event) {
      if ("code" in e) {
        finalError = new Error(`WebSocket closed: (${m})`);
      } else {
        finalError = new Error(`WebSocket connection failed (${m})`);
      }
    } else if (e instanceof Error) {
      finalError = e;
    } else {
      finalError = new Error(`Unknown Error (${m})`);
    }

    captureException(finalError);
  },
});
