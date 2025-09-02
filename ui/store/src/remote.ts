"use client";

import { createElement } from "react";
import { Hydrate } from "./hydrate.js";
import { createConfig } from "./createConfig.js";
import { graphql } from "@left-curve/dango";
import { DangoStoreContext } from "./context.js";

import type { Chain } from "@left-curve/dango/types";
import type { AnyCoin } from "./types/coin.js";

export interface WindowDangoStore extends Window {
  dango: {
    chain: Chain;
    coins: Record<string, AnyCoin>;
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
