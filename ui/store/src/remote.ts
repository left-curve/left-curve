"use client";

import { uid } from "@left-curve/dango/utils";
import { deserializeJson, serializeJson } from "@left-curve/dango/encoding";

import type { Chain } from "@left-curve/dango/types";
import type { AnyCoin } from "./types/coin.js";
import type { RemoteResponse } from "./types/remote.js";
import type { Connection } from "./types/connector.js";

export interface WindowDangoStore extends Window {
  dango: {
    chain: Chain;
    coins: Record<string, AnyCoin>;
    connection?: Omit<Connection, "connector">;
  };
  ReactNativeWebView: {
    postMessage: (message: string) => void;
  };
}

declare let window: WindowDangoStore;

export const requestRemote = async <T = unknown>(
  method: string,
  ...args: unknown[]
): Promise<T> => {
  const id = uid();

  const message = {
    id,
    method,
    args,
  };

  return await new Promise((resolve, reject) => {
    const receiveResponse = (e: MessageEvent) => {
      const message = deserializeJson<RemoteResponse<T>>(e.data);

      if (!message || message.type !== "dango-remote") {
        return;
      }

      if (message.id !== id) return;

      window.removeEventListener("message", receiveResponse);

      const { data, error } = message;

      if (error) reject(error);

      resolve(data as T);
    };

    window.addEventListener("message", receiveResponse);

    window.ReactNativeWebView?.postMessage(serializeJson(message));
  });
};
