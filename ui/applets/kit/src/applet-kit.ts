import { deserializeJson, serializeJson } from "@left-curve/dango/encoding";
import type { ActionRequest, ActionResponse } from "./types/actions";
import type { Events, EventsType } from "./types/subscriptions";

export function initAppletKit() {
  const KIT_VERSION = 1;
  const listeners: {
    [K in EventsType]: Set<(p: Extract<Events, { type: K }>) => void>;
  } = {
    balances: new Set(),
    account: new Set(),
  };

  function sendMessage<T extends Record<string, any> = Record<string, any>>(message: T) {
    if (window.ReactNativeWebView) {
      window.ReactNativeWebView.postMessage(serializeJson(message));
    } else {
      const [origin] = document.location.ancestorOrigins;
      window.parent.postMessage(serializeJson(message), origin);
    }
    throw Error("we couldn't find a transport to send the message");
  }

  async function sendActionMessage<T extends Record<string, any> = Record<string, any>>(
    message: Omit<ActionRequest<T>, "id">,
  ) {
    const id = crypto.randomUUID();

    return await new Promise((resolve, reject) => {
      const handleResponse = (e: MessageEvent) => {
        const message = deserializeJson<ActionResponse>(e.data);

        if (message?.id !== id) return;

        window.removeEventListener("message", handleResponse);

        if (message.status === "success") resolve(message.payload);
        else reject(message.payload);
      };

      window.addEventListener("message", handleResponse);

      sendMessage({ id, ...message });
    });
  }

  function subscribe<subscription extends Events["type"]>(
    subscription: subscription,
    listener: (e: Extract<Events, { type: subscription }>) => void,
  ) {
    const target = listeners[subscription];
    if (!target) throw new Error(`unknown subscription type: ${subscription}`);
    target.add(listener);
    return () => target.delete(listener);
  }

  const AppletKit = {
    sendActionMessage,
    subscribe,
    KIT_VERSION,
  };

  return AppletKit;
}
