"use client";

import { createElement } from "react";
import { Hydrate } from "./hydrate.js";
import { createConfig } from "./createConfig.js";
import { graphql } from "@left-curve/dango";
import { DangoStoreContext } from "./context.js";
import { uid } from "@left-curve/dango/utils";
import { deserializeJson, serializeJson } from "@left-curve/dango/encoding";

import type { Chain } from "@left-curve/dango/types";
import type { AnyCoin } from "./types/coin.js";
import type { RemoteResponse } from "./types/remote.js";

export interface WindowDangoStore extends Window {
  dango: {
    chain: Chain;
    coins: Record<string, AnyCoin>;
  };
  ReactNativeWebView: {
    postMessage: (message: string) => void;
  };
}

declare let window: WindowDangoStore;

export const DangoRemoteProvider: React.FC<React.PropsWithChildren> = (parameters) => {
  const { children } = parameters;

  const chain = window.dango.chain;

  const config = createConfig({
    chain,
    transport: graphql(chain.urls.indexer, { batch: true }),
    coins: window.dango.coins,
    ssr: false,
    connectors: [],
  });

  return createElement(
    Hydrate,
    { config, reconnectOnMount: false },
    createElement(DangoStoreContext.Provider, { value: config }, children),
  );
};

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
