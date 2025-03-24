"use client";

import { type ReactElement, useEffect, useRef } from "react";
import { rehydrate } from "./rehydrate.js";
import type { Config, State } from "./types/store.js";

export type HydrateProps = {
  config: Config;
  initialState?: State;
  reconnectOnMount?: boolean;
};

export function Hydrate(parameters: React.PropsWithChildren<HydrateProps>) {
  const { children, config, initialState, reconnectOnMount = true } = parameters;

  const { onMount } = rehydrate(config, {
    initialState,
    reconnectOnMount,
  });

  // Hydrate for non-SSR
  if (!config._internal.ssr) onMount();

  // Hydrate for SSR
  const active = useRef(true);
  useEffect(() => {
    if (!active.current) return;
    if (!config._internal.ssr) return;
    onMount();
    return () => {
      active.current = false;
    };
  }, []);

  return children as ReactElement;
}
