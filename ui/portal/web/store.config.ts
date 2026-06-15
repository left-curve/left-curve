import {
  createAsyncStorage,
  createConfig,
  createTransport,
  debug,
  passkey,
  session,
  privy,
} from "@left-curve/store";
import { createIndexedDBStorage } from "./storage.config";
import { coins } from "@left-curve/foundation/coins";

import type { Config } from "@left-curve/store/types";

import { PRIVY_APP_ID, PRIVY_CLIENT_ID } from "~/constants";
import { reportStoreError } from "~/app.sentry";

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
    debug(),
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
  onError: reportStoreError,
});
