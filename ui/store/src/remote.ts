"use client";

import { createElement } from "react";
import { Hydrate } from "./hydrate.js";
import { createConfig } from "./createConfig.js";
import { graphql } from "@left-curve/dango";
import { DangoStoreContext } from "./context.js";

import type { Chain } from "@left-curve/dango/types";
import type { AnyCoin } from "./types/coin.js";

declare global {
  interface Window {
    dango_store: {
      chain: Chain;
      coins: Record<string, AnyCoin>;
    };
  }
}

export const DangoRemoteProvider: React.FC<React.PropsWithChildren> = (parameters) => {
  const { children } = parameters;

  const chain = window.dango_store.chain;

  const config = createConfig({
    chain,
    transport: graphql(chain.urls.indexer, { batch: true }),
    coins: window.dango_store.coins,
    ssr: false,
    connectors: [],
  });

  return createElement(
    Hydrate,
    { config, reconnectOnMount: false },
    createElement(DangoStoreContext.Provider, { value: config }, children),
  );
};
